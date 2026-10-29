//! T24, the device plugin's NIC pair: off when this process does not run on
//! DPDK, ok when no pair was granted, a warning naming the exact keys still
//! empty when the pair is missing or inconsistent, and the two shortest copy
//! lines when all three sender values are set. It never returns Unknown, so
//! it never joins `unknown_stops_require`.
#![allow(missing_docs)]

use ergon_tune::{Severity, Status, t24_dpdk_nic_pair};

fn env<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |name| {
        vars.iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| (*v).to_owned())
    }
}

const PCI: &str = "RUSTERON_DPDK_SENDER_PCI";
const CIDR: &str = "RUSTERON_DPDK_SENDER_IPV4_CIDR";
const GATEWAY: &str = "RUSTERON_DPDK_SENDER_GATEWAY";
const FSTACK: &str =
    "fstack-spin,fstack_addr=10.0.1.20,fstack_gateway=10.0.1.1,fstack_pci=0000:00:06.0";
const SMOLTCP: &str = "port=0000:00:06.0,ip=10.0.1.20/24,eal=-a 0000:00:06.0";
const PAIR: [(&str, &str); 3] = [
    (PCI, "0000:00:06.0"),
    (CIDR, "10.0.1.20/24"),
    (GATEWAY, "10.0.1.1"),
];

#[test]
fn t24_is_ok_when_the_process_does_not_run_on_dpdk() {
    // Even a complete pair does not apply: kernel backends and the socket
    // transport never read it.
    let v = t24_dpdk_nic_pair(false, env(&PAIR));
    assert_eq!(v.id, "T24");
    assert_eq!(v.status, Status::Ok, "{v}");
    assert!(!v.is_unknown(), "{v}");
    assert!(v.message.contains("not an ergon-io DPDK backend"), "{v}");
    assert!(!v.message.contains("10.0.1."), "{v}");
}

#[test]
fn t24_is_ok_when_the_device_plugin_has_not_granted_a_pair() {
    let v = t24_dpdk_nic_pair(true, env(&[]));
    assert_eq!(v.status, Status::Ok, "{v}");
    assert!(!v.is_unknown(), "{v}");
    assert!(!v.is_finding(), "{v}");
    assert!(
        v.message.contains("device plugin has not granted a pair"),
        "{v}"
    );
    // Off, not guessed: no address of any host appears.
    assert!(!v.message.contains("10.0.1."), "{v}");
    assert!(!v.message.contains("0000:00"), "{v}");
}

#[test]
fn t24_warns_with_the_exact_keys_still_empty() {
    let v = t24_dpdk_nic_pair(true, env(&[(CIDR, "10.0.1.20/24")]));
    assert!(v.is_warning(), "{v}");
    assert!(!v.is_unknown(), "{v}");
    for part in [PCI, GATEWAY] {
        assert!(v.message.contains(part), "{part} missing from {v}");
    }
    assert!(!v.message.contains(CIDR), "a set key is not empty: {v}");
    // An empty value is an empty key.
    let v = t24_dpdk_nic_pair(
        true,
        env(&[(PCI, ""), (CIDR, "10.0.1.20/24"), (GATEWAY, "10.0.1.1")]),
    );
    assert!(v.is_warning(), "{v}");
    assert!(v.message.contains(PCI), "{v}");
    // No copy lines while a key is empty.
    assert!(!v.message.contains(FSTACK), "{v}");
    assert!(!v.message.contains(SMOLTCP), "{v}");
}

#[test]
fn t24_warns_on_an_inconsistent_value() {
    let gateway = t24_dpdk_nic_pair(
        true,
        env(&[
            (PCI, "0000:00:06.0"),
            (CIDR, "10.0.1.20/24"),
            (GATEWAY, "not-an-ip"),
        ]),
    );
    assert!(gateway.is_warning(), "{gateway}");
    assert!(gateway.message.contains(GATEWAY), "{gateway}");
    assert!(gateway.message.contains("not-an-ip"), "{gateway}");
    assert!(!gateway.message.contains(FSTACK), "{gateway}");

    let cidr = t24_dpdk_nic_pair(
        true,
        env(&[
            (PCI, "0000:00:06.0"),
            (CIDR, "10.0.1.999/24"),
            (GATEWAY, "10.0.1.1"),
        ]),
    );
    assert!(cidr.is_warning(), "{cidr}");
    assert!(cidr.message.contains(CIDR), "{cidr}");
    assert!(cidr.message.contains("10.0.1.999/24"), "{cidr}");

    let pci = t24_dpdk_nic_pair(
        true,
        env(&[
            (PCI, "0000:00:zz.0"),
            (CIDR, "10.0.1.20/24"),
            (GATEWAY, "10.0.1.1"),
        ]),
    );
    assert!(pci.is_warning(), "{pci}");
    assert!(pci.message.contains(PCI), "{pci}");
    assert!(pci.message.contains("0000:00:zz.0"), "{pci}");
}

#[test]
fn t24_prints_the_two_shortest_lines_when_the_pair_is_set() {
    let v = t24_dpdk_nic_pair(true, env(&PAIR));
    assert_eq!(v.status, Status::Finding, "{v}");
    assert!(
        !v.is_warning(),
        "a complete pair must not refuse a require start: {v}"
    );
    assert!(!v.is_unknown(), "{v}");
    assert_eq!(v.severity, Severity::Info, "{v}");
    assert!(v.message.contains(FSTACK), "{v}");
    assert!(v.message.contains(SMOLTCP), "{v}");
    // Read only: the lines are printed, not exported.
    assert!(v.message.contains("copies nothing"), "{v}");
}

#[test]
fn t24_accepts_a_dpdk_device_name_and_a_real_pci_address() {
    let name = t24_dpdk_nic_pair(
        true,
        env(&[
            (PCI, "net_tap0"),
            (CIDR, "10.0.1.20/24"),
            (GATEWAY, "10.0.1.1"),
        ]),
    );
    assert_eq!(name.status, Status::Finding, "{name}");
    assert!(name.message.contains("port=net_tap0"), "{name}");

    let upper = t24_dpdk_nic_pair(
        true,
        env(&[
            (PCI, "0000:81:00.4"),
            (CIDR, "10.0.1.20/24"),
            (GATEWAY, "10.0.1.1"),
        ]),
    );
    assert_eq!(upper.status, Status::Finding, "{upper}");
    assert!(upper.message.contains("fstack_pci=0000:81:00.4"), "{upper}");
}

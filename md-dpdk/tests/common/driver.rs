//! A real Aeron driver over the fake NICs, run by the test's own thread: the
//! bundled binary's environment, and a wire carrying what one fake port sends
//! to the other.

use super::transport::{RECEIVER_IP, RX_GOOD, SENDER_IP};
use super::*;

use rusteron_media_driver::Aeron;
use std::os::raw::c_int;

/// Frames the fake ports transmitted, carried to the other port, as a wire
/// between the sender NIC (port 0, 10.0.0.1) and the receiver NIC (port 1,
/// 10.0.1.1) would.
pub struct Wire {
    pub next: c_int,
    /// Frames carried from the sender NIC to the receiver NIC, and back.
    pub carried: [usize; 2],
    /// The NIC receive timestamp each carried frame arrives with (ns, 0:
    /// none), as an ENA in a precision-time placement group stamps them.
    pub hw_rx_ns: u64,
}

impl Wire {
    pub const fn new() -> Self {
        Self {
            next: 0,
            carried: [0, 0],
            hw_rx_ns: 0,
        }
    }

    pub fn carry(&mut self) -> usize {
        let count = unsafe { rusteron_dpdk_fake_capture_count() };
        assert!(count < 1024, "the fake's capture is full: the wire would go quiet");
        let mut carried = 0;
        while self.next < count {
            let mut c = unsafe { std::mem::zeroed::<FakeCapture>() };
            assert_eq!(unsafe { rusteron_dpdk_fake_capture_at(self.next, &mut c) }, 0);
            self.next += 1;
            let frame = &c.data[..c.len as usize];
            if frame.len() < 34 || frame[12..14] != [0x08, 0x00] {
                continue; // IPv4 only
            }
            let to = match (c.port_id, [frame[30], frame[31], frame[32], frame[33]]) {
                (0, ip) if ip == RECEIVER_IP => 1,
                (1, ip) if ip == SENDER_IP => 0,
                _ => continue,
            };
            let rc = unsafe {
                rusteron_dpdk_fake_rx_inject_hw_ts(to, frame.as_ptr(), frame.len(), RX_GOOD, 1, self.hw_rx_ns)
            };
            assert_eq!(rc, 0, "the fake port {to}'s receive queue is full");
            self.carried[usize::from(c.port_id)] += 1;
            carried += 1;
        }
        carried
    }
}

/// The canonical DPDK environment, as the bundled binary reads it.
pub fn dpdk_env() -> Vec<EnvGuard> {
    [
        ("RUSTERON_MEDIA_DRIVER_TRANSPORT", "dpdk-ena"),
        ("RUSTERON_DPDK_FILE_PREFIX", "rusteron-ena"),
        ("RUSTERON_DPDK_SENDER_PCI", "0000:00:01.0"),
        ("RUSTERON_DPDK_SENDER_IPV4_CIDR", "10.0.0.1/24"),
        ("RUSTERON_DPDK_SENDER_GATEWAY", "10.0.0.254"),
        ("RUSTERON_DPDK_RECEIVER_PCI", "0000:00:02.0"),
        ("RUSTERON_DPDK_RECEIVER_IPV4_CIDR", "10.0.1.1/24"),
        ("RUSTERON_DPDK_RECEIVER_GATEWAY", "10.0.1.254"),
        ("RUSTERON_DPDK_MAX_AERON_MTU", "1408"),
        ("AERON_SENDER_WILDCARD_PORT_RANGE", "20000 20999"),
        ("AERON_RECEIVER_WILDCARD_PORT_RANGE", "21000 21999"),
        ("AERON_MTU_LENGTH", "1408"),
        ("AERON_TERM_BUFFER_LENGTH", "65536"),
        ("AERON_IPC_TERM_BUFFER_LENGTH", "65536"),
        ("AERON_DIR_DELETE_ON_START", "true"),
    ]
    .into_iter()
    .map(|(k, v)| EnvGuard::set(k, v))
    .collect()
}

/// One duty cycle of the process: the driver's agents, the client's
/// conductor, then the wire.
pub fn cycle(driver: &ergon_md_dpdk::EnvMediaDriver, aeron: &Aeron, wire: &mut Wire) {
    driver.do_work().unwrap();
    aeron.main_do_work().unwrap();
    wire.carry();
}

/// Each NIC's next hop for the other's subnet is the other NIC's MAC.
pub fn seed_next_hops(native: *mut std::os::raw::c_void) {
    for (gateway, mac) in [("10.0.0.254", [2, 0, 0, 0, 0, 2]), ("10.0.1.254", [2, 0, 0, 0, 0, 1])] {
        let ip = std::ffi::CString::new(gateway).unwrap();
        assert_eq!(
            unsafe { rusteron_dpdk_transport_test_arp_seed(native, ip.as_ptr(), mac.as_ptr()) },
            0
        );
    }
}

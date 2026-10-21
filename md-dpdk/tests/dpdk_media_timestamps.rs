//! Media receive timestamps (`media-rcv-ts-offset`) over the DPDK transport.
//!
//! Aeron's kernel transport has the kernel stamp each datagram of a channel
//! that asks for them (`SO_TIMESTAMPNS`, `CLOCK_REALTIME`) and flags the
//! transport, and the image writes the stamp at the channel's offset only on
//! that flag. Over DPDK a channel asking is flagged the same way and every
//! datagram of its NIC comes with a stamp: the NIC's hardware receive time
//! when `RUSTERON_DPDK_RX_HW_TIMESTAMP=1`, else `CLOCK_REALTIME` read once a
//! receive burst. A NIC without such a channel reads no clock for them and
//! passes none. Host-local channels, on kernel sockets, get the kernel's.
//! Last, a real Aeron driver writes the stamp into a subscriber's message.
//!
//! Linux x86_64 or aarch64, against the DPDK-free fakes.
#![allow(
    missing_docs,
    unsafe_code,
    unsafe_op_in_unsafe_fn,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::restriction
)]
#![cfg(all(
    feature = "dpdk",
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]

mod common;
use common::driver::{Wire, cycle, dpdk_env, seed_next_hops};
use common::transport::{Harness, Poller, RECEIVER_IP, RECEIVER_PORT, RX_GOOD, RealMmsghdr, frame_to_receiver};
use common::*;

use ergon_md_dpdk::driver::ConfiguredDriver;
use rusteron_media_driver::bindings::*;
use rusteron_media_driver::{Aeron, AeronContext, AeronDriverContext, AeronIdleStrategyKind, Handlers};
use serial_test::serial;

use std::ffi::CString;
use std::os::raw::{c_int, c_void};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

unsafe extern "C" {
    fn rusteron_dpdk_active_runtime() -> *mut c_void;
}

/// Aeron's `AERON_UDP_CHANNEL_TRANSPORT_MEDIA_RCV_TIMESTAMP` (hardware or
/// software), what its kernel transport flags a timestamping channel with.
const MEDIA_RCV_TIMESTAMP: u32 = 0x3;

/// The media timestamp each received datagram came with (ns), or `None`.
unsafe extern "C" fn stamp_cb(
    _data_paths: *mut aeron_udp_channel_data_paths_t,
    _transport: *mut aeron_udp_channel_transport_t,
    receiver_clientd: *mut c_void,
    _endpoint_clientd: *mut c_void,
    _destination_clientd: *mut c_void,
    _buffer: *mut u8,
    _length: usize,
    _addr: *mut sockaddr_storage,
    media_timestamp: *mut timespec,
) {
    let stamps = &mut *(receiver_clientd as *mut Vec<Option<u64>>);
    stamps.push(if media_timestamp.is_null() {
        None
    } else {
        let ts = &*media_timestamp;
        Some(ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64)
    });
}

/// One poll with real receive buffers (the kernel path receives into them),
/// collecting each datagram's media timestamp.
fn poll_stamps(p: &mut Poller, stamps: &mut Vec<Option<u64>>) -> c_int {
    const VLEN: usize = 16;
    let mut bufs = vec![[0u8; 2048]; VLEN];
    let mut names = vec![unsafe { std::mem::zeroed::<sockaddr_storage>() }; VLEN];
    let mut iovs: Vec<iovec> = bufs
        .iter_mut()
        .map(|b| iovec {
            iov_base: b.as_mut_ptr() as *mut c_void,
            iov_len: b.len(),
        })
        .collect();
    let mut msgvec = vec![unsafe { std::mem::zeroed::<RealMmsghdr>() }; VLEN];
    for i in 0..VLEN {
        msgvec[i].msg_hdr.msg_iov = &mut iovs[i];
        msgvec[i].msg_hdr.msg_iovlen = 1;
        msgvec[i].msg_hdr.msg_name = &mut names[i] as *mut _ as *mut c_void;
        msgvec[i].msg_hdr.msg_namelen = std::mem::size_of::<sockaddr_storage>() as u32;
    }
    let mut bytes = 0i64;
    let rc = p.poll_into(&mut msgvec, Some(stamp_cb), stamps as *mut _ as *mut c_void, &mut bytes);
    // the transport lends msgvec control buffers for a stamped kernel read
    // only for the call: none is left behind for the next caller
    for m in &msgvec {
        assert!(m.msg_hdr.msg_control.is_null() && m.msg_hdr.msg_controllen == 0);
    }
    rc
}

fn inject(port: u16, dport: u16, n: usize, hw_rx_ns: u64) {
    let frame = frame_to_receiver(dport, b"stamp me");
    for _ in 0..n {
        let rc = unsafe { rusteron_dpdk_fake_rx_inject_hw_ts(port, frame.as_ptr(), frame.len(), RX_GOOD, 1, hw_rx_ns) };
        assert_eq!(rc, 0);
    }
}

fn reads() -> u64 {
    unsafe { rusteron_dpdk_test_realtime_reads() }
}

/// A channel asking for media receive timestamps is flagged as Aeron's kernel
/// transport flags it, on the NIC and on a host-local kernel socket alike,
/// and only such a channel: the image writes a stamp only on the flag.
#[test]
#[serial]
fn a_channel_asking_for_media_timestamps_is_flagged_as_aerons_kernel_transport_flags_it() {
    let mut h = Harness::new();
    h.init_rx_with(RECEIVER_IP, 60000, 1472, true);
    assert_eq!(h.rx.timestamp_flags, MEDIA_RCV_TIMESTAMP, "on the NIC");
    let mut plain = h.init_extra_rx_with(RECEIVER_IP, 60001, false);
    assert_eq!(plain.timestamp_flags, 0, "a channel not asking");
    let mut local = h.init_extra_rx_with([127, 0, 0, 1], 0, true);
    assert_eq!(
        local.timestamp_flags, MEDIA_RCV_TIMESTAMP,
        "host-local, on a kernel socket"
    );
    common::transport::close_transport(h.bindings, &mut plain);
    common::transport::close_transport(h.bindings, &mut local);
}

/// Each receive burst on a NIC with such a channel reads `CLOCK_REALTIME`
/// once and stamps its frames with it, as `SO_TIMESTAMPNS` stamps each
/// datagram at the kernel's receive.
#[test]
#[serial]
fn each_burst_stamps_its_frames_with_one_realtime_read() {
    let mut h = Harness::new();
    h.init_rx_with(RECEIVER_IP, 60000, 1472, true);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let mut stamps = Vec::new();

    unsafe { rusteron_dpdk_test_set_realtime_ns(1_700_000_000_123_456_789) };
    let before = reads();
    inject(RECEIVER_PORT, 60000, 3, 0);
    assert_eq!(poll_stamps(&mut p, &mut stamps), 3);
    assert_eq!(stamps, vec![Some(1_700_000_000_123_456_789); 3]);
    assert_eq!(reads() - before, 1, "one read for the burst of three");

    unsafe { rusteron_dpdk_test_set_realtime_ns(1_700_000_001_000_000_007) };
    inject(RECEIVER_PORT, 60000, 1, 0);
    assert_eq!(poll_stamps(&mut p, &mut stamps), 1);
    assert_eq!(stamps[3], Some(1_700_000_001_000_000_007), "the next burst reads again");
    assert_eq!(reads() - before, 2);
    assert_eq!(poll_stamps(&mut p, &mut stamps), 0);
    assert_eq!(reads() - before, 2, "an empty poll reads nothing");
    assert_no_leak();
}

/// Unpinned, a burst's stamp is `CLOCK_REALTIME` in nanoseconds since the
/// epoch, the clock and unit of the kernel's `SO_TIMESTAMPNS`: within a
/// second of the system's wall-clock time (a monotonic clock counts from
/// boot, and a unit slip is off by orders of magnitude).
#[test]
#[serial]
fn a_bursts_stamp_is_clock_realtime_in_nanoseconds() {
    let mut h = Harness::new();
    h.init_rx_with(RECEIVER_IP, 60000, 1472, true);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let now = || SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
    let mut stamps = Vec::new();
    let before = now();
    inject(RECEIVER_PORT, 60000, 1, 0);
    assert_eq!(poll_stamps(&mut p, &mut stamps), 1);
    let after = now();
    let ns = stamps[0].expect("stamped");
    assert!(
        ns + 1_000_000_000 > before && ns < after + 1_000_000_000,
        "CLOCK_REALTIME at the burst: {ns}, between {before} and {after}"
    );
    assert_no_leak();
}

/// A NIC without a channel asking reads no clock for them and passes none,
/// and one stops reading once its last such channel closes.
#[test]
#[serial]
fn a_nic_without_such_a_channel_reads_no_clock_and_passes_none() {
    let mut h = Harness::new();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut stamping = h.init_extra_rx_with(RECEIVER_IP, 60001, true);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    p.add(&mut *stamping);
    unsafe { rusteron_dpdk_test_set_realtime_ns(42) };
    let mut stamps = Vec::new();

    // While one channel on the NIC asks, every frame of a burst carries the
    // stamp (Aeron writes it only into the channel that asked).
    inject(RECEIVER_PORT, 60000, 2, 0);
    assert_eq!(poll_stamps(&mut p, &mut stamps), 2);
    assert_eq!(stamps, vec![Some(42), Some(42)]);

    p.remove(&mut *stamping);
    common::transport::close_transport(h.bindings, &mut stamping);
    stamps.clear();
    let before = reads();
    inject(RECEIVER_PORT, 60000, 2, 0);
    assert_eq!(poll_stamps(&mut p, &mut stamps), 2);
    assert_eq!(stamps, vec![None, None]);
    assert_eq!(reads(), before, "no clock read for a NIC without such a channel");
    assert_no_leak();
}

/// With `RUSTERON_DPDK_RX_HW_TIMESTAMP=1`, a frame the NIC stamped carries its
/// arrival time (the PHC's UTC nanoseconds, `CLOCK_REALTIME`'s scale); one it
/// did not, the burst's `CLOCK_REALTIME`.
#[test]
#[serial]
fn nic_receive_timestamps_are_the_media_timestamp() {
    let mut h = Harness::with_rx_hw_timestamp();
    h.init_rx_with(RECEIVER_IP, 60000, 1472, true);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    unsafe { rusteron_dpdk_test_set_realtime_ns(1_700_000_000_500_000_000) };
    let mut stamps = Vec::new();
    inject(RECEIVER_PORT, 60000, 1, 1_700_000_000_499_998_765);
    inject(RECEIVER_PORT, 60000, 1, 0);
    assert_eq!(poll_stamps(&mut p, &mut stamps), 2);
    assert_eq!(
        stamps,
        vec![Some(1_700_000_000_499_998_765), Some(1_700_000_000_500_000_000)]
    );
    assert_no_leak();
}

/// A host-local channel asking for them gets the kernel's receive timestamp
/// (`SO_TIMESTAMPNS`), as on Aeron's kernel transport, and one not asking none.
#[test]
#[serial]
fn a_host_local_channel_gets_the_kernels_receive_timestamp() {
    let mut h = Harness::new();
    // pinned, so a stamp the transport made up itself would read exactly this
    unsafe { rusteron_dpdk_test_set_realtime_ns(7) };
    let peer = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    for asking in [true, false] {
        let mut t = h.init_extra_rx_with([127, 0, 0, 1], 0, asking);
        let bound = Harness::bind_addr_and_port(h.bindings, &mut t);
        let mut p = h.poller();
        p.add(&mut *t);
        let sent = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
        peer.send_to(b"hello", &bound).unwrap();
        let mut stamps = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(2);
        while stamps.is_empty() && Instant::now() < deadline {
            poll_stamps(&mut p, &mut stamps);
        }
        assert_eq!(stamps.len(), 1, "asking {asking}: received over the kernel");
        if asking {
            let ns = stamps[0].expect("the kernel's stamp");
            assert!(
                ns + 1_000_000_000 > sent && ns < sent + 1_000_000_000,
                "the kernel's CLOCK_REALTIME at receive: {ns}, sent at {sent}"
            );
        } else {
            assert_eq!(stamps[0], None);
        }
        drop(p);
        common::transport::close_transport(h.bindings, &mut t);
    }
}

/// A real Aeron driver over the fake NICs writes each message's media receive
/// timestamp at the subscription channel's `media-rcv-ts-offset` (here the
/// first 8 bytes of the message, little-endian): `CLOCK_REALTIME` from the
/// receive burst, or the NIC's with `RUSTERON_DPDK_RX_HW_TIMESTAMP=1`. A
/// channel without the offset gets nothing written.
#[test]
#[serial]
fn a_real_image_writes_the_media_timestamp_at_the_channels_offset() {
    const REALTIME: u64 = 1_760_000_000_111_222_333;
    const NIC: u64 = 1_760_000_000_111_000_444;
    for (offset, hw, expected) in [(true, false, REALTIME), (true, true, NIC), (false, false, 0)] {
        // Aeron pins the thread that starts the driver: a thread of its own
        let first = std::thread::spawn(move || first_word_received(offset, hw, REALTIME, NIC))
            .join()
            .unwrap_or_else(|_| panic!("offset {offset}, hw {hw}: the driver's thread failed"));
        let want = if 0 == expected { 0xa5a5_a5a5_a5a5_a5a5 } else { expected };
        assert_eq!(first, want, "offset {offset}, hw {hw}");
    }
}

/// Publish one 16-byte message whose first word is 0xa5.., receive it, and
/// return its first word as the subscriber saw it.
fn first_word_received(offset: bool, hw: bool, realtime: u64, nic: u64) -> u64 {
    let env = TestEnv::new();
    env.eal_skip();
    let _dpdk = dpdk_env();
    let _hw = hw.then(|| {
        env.set_rx_timestamp_ok(true);
        EnvGuard::set("RUSTERON_DPDK_RX_HW_TIMESTAMP", "1")
    });
    unsafe { rusteron_dpdk_test_set_realtime_ns(realtime) };
    let dir = format!(
        "{}/aeron-dpdk-media-ts-{offset}-{hw}-{}",
        std::env::temp_dir().display(),
        std::process::id()
    );
    let context = AeronDriverContext::new().unwrap();
    context.set_dir(&CString::new(dir.clone()).unwrap()).unwrap();
    context
        .set_threading_mode(aeron_threading_mode_t::AERON_THREADING_MODE_INVOKER)
        .unwrap();
    context.set_conductor_cpu_affinity(first_allowed_cpu()).unwrap();
    context
        .set_shared_idle_strategy_kind(AeronIdleStrategyKind::NoOp)
        .unwrap();
    let configured = ConfiguredDriver::from_env(context).unwrap_or_else(|e| panic!("{e}"));
    seed_next_hops(unsafe { rusteron_dpdk_active_runtime() });
    let driver = configured.start().unwrap();

    let client = AeronContext::new().unwrap();
    client.set_dir(&CString::new(dir.clone()).unwrap()).unwrap();
    client.set_use_conductor_agent_invoker(true).unwrap();
    let aeron = Aeron::new(&client).unwrap();
    aeron.start().unwrap();
    let mut wire = Wire::new();
    wire.hw_rx_ns = if hw { nic } else { 0 };
    let deadline = Instant::now() + Duration::from_secs(20);
    let until = |what: &str, wire: &mut Wire, done: &mut dyn FnMut() -> bool| {
        while !done() {
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            cycle(&driver, &aeron, wire);
        }
    };

    let sub_channel = CString::new(if offset {
        "aeron:udp?endpoint=10.0.1.1:40456|media-rcv-ts-offset=0"
    } else {
        "aeron:udp?endpoint=10.0.1.1:40456"
    })
    .unwrap();
    let pub_channel = CString::new("aeron:udp?endpoint=10.0.1.1:40456").unwrap();
    let pending_sub = aeron
        .async_add_subscription(&sub_channel, 1001, Handlers::NONE, Handlers::NONE)
        .unwrap();
    let pending_pub = aeron.async_add_exclusive_publication(&pub_channel, 1001).unwrap();
    let mut subscription = None;
    let mut publication = None;
    until("the publication and subscription", &mut wire, &mut || {
        if subscription.is_none() {
            subscription = pending_sub.poll().unwrap();
        }
        if publication.is_none() {
            publication = pending_pub.poll().unwrap();
        }
        subscription.is_some() && publication.is_some()
    });
    let (subscription, publication) = (subscription.unwrap(), publication.unwrap());
    until("the image and the connection", &mut wire, &mut || {
        publication.is_connected() && subscription.image_at_index(0).is_some()
    });

    let mut message = [0xa5u8; 16];
    message[8..].copy_from_slice(b"payload!");
    until("the offer", &mut wire, &mut || publication.offer(&message).is_ok());
    let mut received = Vec::new();
    until("the message", &mut wire, &mut || {
        subscription
            .poll_fn(|data, _header| received.extend_from_slice(data), 10)
            .unwrap();
        !received.is_empty()
    });
    assert_eq!(&received[8..], b"payload!", "the rest of the message is untouched");

    drop((subscription, publication, pending_sub, pending_pub));
    drop(aeron);
    drop(client);
    driver.abandon().unwrap();
    drop(env);
    let _ = std::fs::remove_dir_all(&dir);
    u64::from_le_bytes(received[..8].try_into().unwrap())
}

/// The first CPU this process may run on: the driver's one thread goes there.
fn first_allowed_cpu() -> i32 {
    let list = std::fs::read_to_string("/proc/self/status").unwrap();
    let cpus = list
        .lines()
        .find_map(|l| l.strip_prefix("Cpus_allowed_list:"))
        .unwrap()
        .trim();
    cpus.split([',', '-']).next().unwrap().parse().unwrap()
}

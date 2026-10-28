//! The lab's per-stage trace (`RUSTERON_DPDK_STAGE_TRACE=<N>`,
//! `RUSTERON_DPDK_STAGE_TRACE_FILE`) and in-process capture
//! (`RUSTERON_DPDK_CAPTURE_FILE`, `RUSTERON_DPDK_CAPTURE_WINDOW_MS`), against
//! the fake ports.
//!
//! A message is an aeron_lat one in an Aeron data frame (seq at payload byte
//! 0, kind at 28); every Nth by its seq is stamped at each stage it passes:
//! the send's entry (`tx-send`), around the `tx_burst` that hands it to the
//! NIC (`tx-call`, `tx-done`), the receive burst's return (`rx-burst`), the
//! NIC's arrival (`rx-hw`, with RX timestamps) and around Aeron's receive
//! callback (`rx-dispatch`, `rx-inserted`). Each row carries the frame's
//! stream id, the message's seq and the frame's term offset, so the two hosts'
//! rows of one message line up. The socket transport's wrapped bindings stamp
//! the same stages around its sends and receive callbacks.
//!
//! Linux x86_64 or aarch64, and requires libdpdk >= 23.11 (the `dpdk` feature).
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
use common::transport::*;
use common::*;

use rusteron_media_driver::bindings::*;
use serial_test::serial;

use std::collections::BTreeMap;
use std::os::raw::{c_int, c_void};

unsafe extern "C" {
    fn rusteron_dpdk_lab_open(kernel: c_int) -> c_int;
    fn rusteron_dpdk_lab_drain();
    fn rusteron_dpdk_lab_close();
    fn rusteron_dpdk_lab_test_set_no_thread(no_thread: c_int);
    fn rusteron_dpdk_lab_test_set_ring_records(records: u32);
    fn rusteron_dpdk_stage_wrap_bindings(
        inner: *const aeron_udp_channel_transport_bindings_stct,
        out: *mut aeron_udp_channel_transport_bindings_stct,
    );
}

const STREAM: i32 = 1001;
const ECHO_STREAM: i32 = 11001;

/// A fresh file path for `name` (removed if an earlier run left it).
fn path(name: &str) -> String {
    let p = std::env::temp_dir().join(format!("ergon-md-dpdk-stages-{}-{name}", std::process::id()));
    let _ = std::fs::remove_file(&p);
    p.to_string_lossy().into_owned()
}

/// An Aeron data frame (32-byte header) holding aeron_lat's 64-byte message.
fn data_frame(stream_id: i32, term_offset: i32, seq: u64, kind: u32) -> Vec<u8> {
    let mut f = vec![0u8; 96];
    f[0..4].copy_from_slice(&96i32.to_le_bytes());
    f[4] = 0; // version
    f[5] = 0xc0; // begin | end
    f[6..8].copy_from_slice(&1u16.to_le_bytes()); // DATA
    f[8..12].copy_from_slice(&term_offset.to_le_bytes());
    f[12..16].copy_from_slice(&7i32.to_le_bytes()); // session
    f[16..20].copy_from_slice(&stream_id.to_le_bytes());
    f[20..24].copy_from_slice(&3i32.to_le_bytes()); // term id
    f[32..40].copy_from_slice(&seq.to_le_bytes());
    f[40..48].copy_from_slice(&0x1111u64.to_le_bytes()); // send_tsc
    f[56..60].copy_from_slice(&0u32.to_le_bytes()); // stream index
    f[60..64].copy_from_slice(&kind.to_le_bytes());
    f
}

/// A status message (24 bytes): never stamped.
fn status_message(stream_id: i32) -> Vec<u8> {
    let mut f = vec![0u8; 36];
    f[0..4].copy_from_slice(&36i32.to_le_bytes());
    f[6..8].copy_from_slice(&3u16.to_le_bytes()); // SM
    f[12..16].copy_from_slice(&stream_id.to_le_bytes());
    f
}

#[derive(Debug, Clone, PartialEq)]
struct Row {
    stage: String,
    tsc: u64,
    stream_id: i32,
    seq: u64,
    term_offset: i32,
    value: u32,
    ring: u32,
}

/// The file's rows, and its `#` lines.
fn read(p: &str) -> (Vec<Row>, Vec<String>) {
    let text = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{p}: {e}"));
    let mut rows = Vec::new();
    let mut notes = Vec::new();
    let mut header = false;
    for l in text.lines() {
        if l.starts_with('#') {
            notes.push(l.to_string());
            continue;
        }
        if !header {
            assert_eq!(l, "stage,tsc,stream_id,seq,term_offset,value,ring", "{text}");
            header = true;
            continue;
        }
        let f: Vec<&str> = l.split(',').collect();
        assert_eq!(f.len(), 7, "{l}");
        rows.push(Row {
            stage: f[0].into(),
            tsc: f[1].parse().unwrap(),
            stream_id: f[2].parse().unwrap(),
            seq: f[3].parse().unwrap(),
            term_offset: f[4].parse().unwrap(),
            value: f[5].parse().unwrap(),
            ring: f[6].parse().unwrap(),
        });
    }
    assert!(header, "no header: {text}");
    (rows, notes)
}

/// Each message's (stream, seq) stages, in file order.
fn by_message(rows: &[Row]) -> BTreeMap<(i32, u64), Vec<Row>> {
    let mut m: BTreeMap<(i32, u64), Vec<Row>> = BTreeMap::new();
    for r in rows {
        m.entry((r.stream_id, r.seq)).or_default().push(r.clone());
    }
    m
}

fn stage<'a>(rows: &'a [Row], name: &str) -> &'a Row {
    rows.iter()
        .find(|r| r.stage == name)
        .unwrap_or_else(|| panic!("no {name}: {rows:?}"))
}

/// A sending harness with the trace every `every` at `file`, its next hop known.
fn sender(every: &str, file: &str) -> Harness {
    let _e = (
        EnvGuard::set("RUSTERON_DPDK_STAGE_TRACE", every),
        EnvGuard::set("RUSTERON_DPDK_STAGE_TRACE_FILE", file),
    );
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.arp_seed(PEER_IP, PEER_MAC);
    h
}

fn receiver(every: &str, file: &str) -> (Harness, Poller) {
    let _e = (
        EnvGuard::set("RUSTERON_DPDK_STAGE_TRACE", every),
        EnvGuard::set("RUSTERON_DPDK_STAGE_TRACE_FILE", file),
    );
    let mut h = Harness::new();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    (h, p)
}

fn poll(h: &mut Harness, p: &mut Poller) -> c_int {
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    p.poll(16, clientd, &mut bytes)
}

/// Each sampled message a send hands to the NIC is stamped at the send's
/// entry, then around its tx_burst, in that order, with its own stream id,
/// seq and term offset; an unsampled one is not, nor a frame that is not an
/// aeron_lat message.
#[test]
#[serial]
fn a_send_stamps_its_sampled_messages_in_order() {
    let file = path("send.csv");
    let mut h = sender("4", &file);
    for seq in 0..8u64 {
        let f = data_frame(STREAM, (seq * 96) as i32, seq, 1);
        assert_eq!(h.send_to((PEER_IP, 40123), &[&f]).0, 1, "{}", last_error());
    }
    // a status message, and a data frame whose payload is no message (kind 0)
    assert_eq!(h.send_to((PEER_IP, 40123), &[&status_message(STREAM)]).0, 1);
    assert_eq!(h.send_to((PEER_IP, 40123), &[&data_frame(STREAM, 0, 8, 0)]).0, 1);
    unsafe { rusteron_dpdk_lab_drain() };
    let (rows, notes) = read(&file);
    let every = format!("# every=4 pid={} transport=dpdk", std::process::id());
    assert!(notes.iter().any(|n| *n == every), "{notes:?}");
    assert!(notes.iter().any(|n| n.starts_with("# clock open tsc=")), "{notes:?}");
    assert!(notes.iter().any(|n| n.starts_with("# ring 0 tid ")), "{notes:?}");
    let m = by_message(&rows);
    assert_eq!(
        m.keys().copied().collect::<Vec<_>>(),
        vec![(STREAM, 0), (STREAM, 4)],
        "{rows:?}"
    );
    for ((_, seq), rs) in &m {
        let names: Vec<&str> = {
            let mut v: Vec<&str> = rs.iter().map(|r| r.stage.as_str()).collect();
            v.sort();
            v
        };
        assert_eq!(names, vec!["tx-call", "tx-done", "tx-send"], "{rs:?}");
        let (send, call, done) = (stage(rs, "tx-send"), stage(rs, "tx-call"), stage(rs, "tx-done"));
        assert!(send.tsc <= call.tsc && call.tsc <= done.tsc, "{rs:?}");
        assert!(send.tsc != 0, "{rs:?}");
        assert!(
            rs.iter()
                .all(|r| r.term_offset == (*seq * 96) as i32 && r.value == 0 && r.ring == 0),
            "{rs:?}"
        );
    }
    unsafe { rusteron_dpdk_lab_close() };
    let (_, notes) = read(&file);
    assert!(notes.iter().any(|n| n.starts_with("# clock close tsc=")), "{notes:?}");
    assert_no_leak();
}

/// Frames the NIC's ring refused are stamped when it takes them, not
/// before: the accepted one at its send; the refused one when the agent's
/// next poll flushes it, then with no `tx-send` (a poll is no send).
#[test]
#[serial]
fn a_refused_frame_is_stamped_when_the_nic_takes_it() {
    let file = path("refused.csv");
    let mut h = sender("4", &file);
    let mut p = h.poller();
    p.add(&mut *h.tx);
    unsafe { rusteron_dpdk_fake_set_tx_burst_cap(1) };
    let (a, b) = (data_frame(STREAM, 0, 0, 1), data_frame(STREAM, 96, 4, 1));
    assert_eq!(h.send_to((PEER_IP, 40123), &[&a, &b]).0, 2, "{}", last_error());
    unsafe { rusteron_dpdk_lab_drain() };
    let m = by_message(&read(&file).0);
    assert_eq!(m.keys().copied().collect::<Vec<_>>(), vec![(STREAM, 0)], "only the one the ring took");
    unsafe { rusteron_dpdk_fake_set_tx_burst_cap(64) };
    let mut bytes = 0i64;
    let mut rx: Vec<RxDatagram> = Vec::new();
    p.poll(16, &mut rx as *mut Vec<RxDatagram> as *mut c_void, &mut bytes);
    unsafe { rusteron_dpdk_lab_drain() };
    let m = by_message(&read(&file).0);
    let late = &m[&(STREAM, 4)];
    let mut names: Vec<&str> = late.iter().map(|r| r.stage.as_str()).collect();
    names.sort();
    assert_eq!(names, vec!["tx-call", "tx-done"], "{late:?}");
    assert!(stage(late, "tx-call").tsc > stage(&m[&(STREAM, 0)], "tx-done").tsc);
    drop(p);
    assert_no_leak();
}

/// Two messages in one datagram (Aeron batches frames up to the MTU): each
/// sampled one is stamped with its own term offset, the others not.
#[test]
#[serial]
fn each_message_of_a_batched_datagram_is_found() {
    let file = path("batch.csv");
    let mut h = sender("2", &file);
    let mut d = data_frame(STREAM, 0, 3, 1);
    d.extend(data_frame(STREAM, 96, 4, 1));
    d.extend(data_frame(STREAM, 192, 6, 1));
    assert_eq!(h.send_to((PEER_IP, 40123), &[&d]).0, 1, "{}", last_error());
    unsafe { rusteron_dpdk_lab_drain() };
    let (rows, _) = read(&file);
    let m = by_message(&rows);
    assert_eq!(
        m.keys().copied().collect::<Vec<_>>(),
        vec![(STREAM, 4), (STREAM, 6)],
        "{rows:?}"
    );
    assert!(m[&(STREAM, 4)].iter().all(|r| r.term_offset == 96));
    assert!(m[&(STREAM, 6)].iter().all(|r| r.term_offset == 192));
    assert_no_leak();
}

/// A received sampled message is stamped at its burst's return, before
/// Aeron's callback and after it returned, in that order; the datagram still
/// reaches the callback whole.
#[test]
#[serial]
fn a_receive_stamps_burst_dispatch_and_insert() {
    let file = path("receive.csv");
    let (mut h, mut p) = receiver("1", &file);
    let f = frame_to_receiver(60000, &data_frame(ECHO_STREAM, 4096, 12, 3));
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, f.as_ptr(), f.len(), RX_GOOD, 1) },
        0
    );
    assert_eq!(poll(&mut h, &mut p), 1);
    assert_eq!(h.rx_dgrams.len(), 1);
    assert_eq!(h.rx_dgrams[0].data, data_frame(ECHO_STREAM, 4096, 12, 3));
    unsafe { rusteron_dpdk_lab_drain() };
    let (rows, _) = read(&file);
    let m = by_message(&rows);
    let rs = &m[&(ECHO_STREAM, 12)];
    assert_eq!(rs.len(), 3, "{rs:?}");
    let (burst, dispatch, inserted) = (
        stage(rs, "rx-burst"),
        stage(rs, "rx-dispatch"),
        stage(rs, "rx-inserted"),
    );
    assert!(
        burst.tsc != 0 && burst.tsc <= dispatch.tsc && dispatch.tsc <= inserted.tsc,
        "{rs:?}"
    );
    assert!(rs.iter().all(|r| r.term_offset == 4096), "{rs:?}");
    assert!(
        !rows.iter().any(|r| r.stage == "rx-hw"),
        "no NIC stamp, no rx-hw: {rows:?}"
    );
    drop(p);
    assert_no_leak();
}

/// With NIC RX timestamps, a sampled message also gets `rx-hw`: the ns from
/// the NIC's arrival to the CLOCK_REALTIME read before its dispatch.
#[test]
#[serial]
fn a_nic_stamped_frame_gets_rx_hw() {
    let file = path("hw.csv");
    let _e = (
        EnvGuard::set("RUSTERON_DPDK_STAGE_TRACE", "1"),
        EnvGuard::set("RUSTERON_DPDK_STAGE_TRACE_FILE", &file),
    );
    let mut h = Harness::with_rx_hw_timestamp();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    unsafe { rusteron_dpdk_test_set_realtime_ns(1_700_000_000_000_007_000) };
    let f = frame_to_receiver(60000, &data_frame(STREAM, 0, 0, 1));
    assert_eq!(
        unsafe {
            rusteron_dpdk_fake_rx_inject_hw_ts(
                RECEIVER_PORT,
                f.as_ptr(),
                f.len(),
                RX_GOOD,
                1,
                1_700_000_000_000_000_000,
            )
        },
        0
    );
    assert_eq!(poll(&mut h, &mut p), 1);
    unsafe { rusteron_dpdk_lab_drain() };
    let (rows, _) = read(&file);
    let hw = rows
        .iter()
        .find(|r| r.stage == "rx-hw")
        .unwrap_or_else(|| panic!("{rows:?}"));
    assert_eq!((hw.stream_id, hw.seq, hw.value), (STREAM, 0, 7_000), "{rows:?}");
    drop(p);
    assert_no_leak();
}

/// A full ring drops the newest records and counts them in the file; the
/// trace goes on once drained.
#[test]
#[serial]
fn a_full_ring_drops_and_counts() {
    let file = path("drops.csv");
    let env = TestEnv::new();
    unsafe { rusteron_dpdk_lab_test_set_ring_records(4) };
    let _e = (
        EnvGuard::set("RUSTERON_DPDK_STAGE_TRACE", "1"),
        EnvGuard::set("RUSTERON_DPDK_STAGE_TRACE_FILE", &file),
    );
    let mut h = Harness::build_after(env, rusteron_dpdk_config_t::valid());
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.arp_seed(PEER_IP, PEER_MAC);
    for seq in 0..3u64 {
        assert_eq!(h.send_to((PEER_IP, 40123), &[&data_frame(STREAM, 0, seq, 1)]).0, 1);
    }
    unsafe { rusteron_dpdk_lab_drain() };
    let (rows, notes) = read(&file);
    assert_eq!(rows.len(), 4, "the ring's four: {rows:?}");
    assert!(
        notes.iter().any(|n| n == "# drops ring 0 5"),
        "3 x 3 stamps, 4 kept: {notes:?}"
    );
    assert_eq!(h.send_to((PEER_IP, 40123), &[&data_frame(STREAM, 0, 9, 1)]).0, 1);
    unsafe { rusteron_dpdk_lab_drain() };
    let (rows, _) = read(&file);
    assert_eq!(rows.iter().filter(|r| r.seq == 9).count(), 3, "{rows:?}");
    assert_no_leak();
}

/// Off unless set: no file, and a malformed value fails the start naming
/// its variable.
#[test]
#[serial]
fn the_environment_is_checked() {
    let file = path("env.csv");
    let _u = (
        EnvGuard::unset("RUSTERON_DPDK_STAGE_TRACE"),
        EnvGuard::unset("RUSTERON_DPDK_CAPTURE_FILE"),
        EnvGuard::unset("RUSTERON_DPDK_CAPTURE_WINDOW_MS"),
    );
    {
        let _f = EnvGuard::set("RUSTERON_DPDK_STAGE_TRACE_FILE", &file);
        let _env = TestEnv::new();
        assert_eq!(unsafe { rusteron_dpdk_lab_open(0) }, -1);
        assert!(
            last_error().contains("RUSTERON_DPDK_STAGE_TRACE_FILE without"),
            "{}",
            last_error()
        );
    }
    for bad in ["3", "0", "x", "2097152"] {
        let _env = TestEnv::new();
        let _e = (
            EnvGuard::set("RUSTERON_DPDK_STAGE_TRACE", bad),
            EnvGuard::set("RUSTERON_DPDK_STAGE_TRACE_FILE", &file),
        );
        assert_eq!(unsafe { rusteron_dpdk_lab_open(0) }, -1, "{bad}");
        assert!(
            last_error().contains(&format!("RUSTERON_DPDK_STAGE_TRACE={bad}")),
            "{}",
            last_error()
        );
        assert_eq!(last_error_code(), 3);
    }
    {
        let _env = TestEnv::new();
        let _e = EnvGuard::set("RUSTERON_DPDK_STAGE_TRACE", "8");
        let _f = EnvGuard::unset("RUSTERON_DPDK_STAGE_TRACE_FILE");
        assert_eq!(unsafe { rusteron_dpdk_lab_open(0) }, -1);
        assert!(
            last_error().contains("needs RUSTERON_DPDK_STAGE_TRACE_FILE"),
            "{}",
            last_error()
        );
    }
    {
        let _env = TestEnv::new();
        let _e = (
            EnvGuard::set("RUSTERON_DPDK_CAPTURE_FILE", &file),
            EnvGuard::set("RUSTERON_DPDK_CAPTURE_WINDOW_MS", "5,10"),
        );
        assert_eq!(unsafe { rusteron_dpdk_lab_open(0) }, -1);
        assert!(
            last_error().contains("RUSTERON_DPDK_CAPTURE_WINDOW_MS=5,10"),
            "{}",
            last_error()
        );
    }
    {
        let _env = TestEnv::new();
        let _e = EnvGuard::set("RUSTERON_DPDK_CAPTURE_FILE", &file);
        assert_eq!(
            unsafe { rusteron_dpdk_lab_open(1) },
            -1,
            "no capture on the socket transport"
        );
        assert!(
            last_error().contains("RUSTERON_DPDK_CAPTURE_FILE is the DPDK transport's"),
            "{}",
            last_error()
        );
    }
    // unset: nothing opened, nothing written, a send unstamped
    let _f = EnvGuard::unset("RUSTERON_DPDK_STAGE_TRACE_FILE");
    let _ = std::fs::remove_file(&file);
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.arp_seed(PEER_IP, PEER_MAC);
    assert_eq!(h.send_to((PEER_IP, 40123), &[&data_frame(STREAM, 0, 0, 1)]).0, 1);
    unsafe { rusteron_dpdk_lab_drain() };
    assert!(!std::path::Path::new(&file).exists());
    assert_no_leak();
}

/// The writer thread drains the rings on its own (every 10 ms), and the
/// close writes the rest.
#[test]
#[serial]
fn the_writer_thread_writes_the_file() {
    let file = path("thread.csv");
    let env = TestEnv::new();
    unsafe { rusteron_dpdk_lab_test_set_no_thread(0) };
    let _e = (
        EnvGuard::set("RUSTERON_DPDK_STAGE_TRACE", "1"),
        EnvGuard::set("RUSTERON_DPDK_STAGE_TRACE_FILE", &file),
    );
    let mut h = Harness::build_after(env, rusteron_dpdk_config_t::valid());
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.arp_seed(PEER_IP, PEER_MAC);
    assert_eq!(h.send_to((PEER_IP, 40123), &[&data_frame(STREAM, 0, 0, 1)]).0, 1);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while read(&file).0.len() < 3 {
        assert!(
            std::time::Instant::now() < deadline,
            "the writer never wrote: {:?}",
            read(&file)
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    unsafe { rusteron_dpdk_lab_close() };
    assert!(read(&file).1.iter().any(|n| n.starts_with("# clock close ")));
    assert_no_leak();
}

// ---- the socket transport's wrapped bindings ----

static mut INNER_SENDS: usize = 0;
static mut INNER_RECV_SEEN: Option<aeron_udp_transport_recv_func_t> = None;

unsafe extern "C" fn inner_send(
    _data_paths: *mut aeron_udp_channel_data_paths_t,
    _transport: *mut aeron_udp_channel_transport_t,
    _address: *mut sockaddr_storage,
    _iov: *mut iovec,
    iov_length: usize,
    _bytes_sent: *mut i64,
) -> c_int {
    INNER_SENDS += 1;
    iov_length as c_int
}

/// The inner receive (Aeron's recvmmsg): hands one ping (stream 1001, seq
/// 0) and one echo (seq 1) to the receive callback it was given.
unsafe extern "C" fn inner_recvmmsg(
    _transport: *mut aeron_udp_channel_transport_t,
    _msgvec: *mut mmsghdr,
    _vlen: usize,
    _bytes_rcved: *mut i64,
    recv_func: aeron_udp_transport_recv_func_t,
    clientd: *mut c_void,
) -> c_int {
    INNER_RECV_SEEN = Some(recv_func);
    let f = recv_func.unwrap();
    for (stream, seq) in [(STREAM, 0u64), (ECHO_STREAM, 1)] {
        let mut d = data_frame(stream, 64, seq, 1);
        f(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            clientd,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            d.as_mut_ptr(),
            d.len(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
    }
    2
}

/// The inner poll, as Aeron's: each transport received through the
/// recvmmsg it was handed (the bindings' own, here the wrapped one).
unsafe extern "C" fn inner_poll(
    _poller: *mut aeron_udp_transport_poller_t,
    msgvec: *mut mmsghdr,
    vlen: usize,
    bytes_rcved: *mut i64,
    recv_func: aeron_udp_transport_recv_func_t,
    recvmmsg_func: aeron_udp_channel_transport_recvmmsg_func_t,
    clientd: *mut c_void,
) -> c_int {
    recvmmsg_func.unwrap()(std::ptr::null_mut(), msgvec, vlen, bytes_rcved, recv_func, clientd)
}

/// The socket transport's sends and receive callbacks are stamped through
/// wrapped bindings that still pass every call on: a send at the call
/// (tx-send and tx-call both) and its return; a datagram at the callback's
/// entry (rx-burst and rx-dispatch both) and its return.
#[test]
#[serial]
fn the_socket_bindings_wrapped_stamp_the_same_stages() {
    let file = path("kernel.csv");
    let _env = TestEnv::new();
    let _e = (
        EnvGuard::set("RUSTERON_DPDK_STAGE_TRACE", "1"),
        EnvGuard::set("RUSTERON_DPDK_STAGE_TRACE_FILE", &file),
    );
    assert_eq!(unsafe { rusteron_dpdk_lab_open(1) }, 0, "{}", last_error());
    let mut inner: aeron_udp_channel_transport_bindings_stct = unsafe { std::mem::zeroed() };
    inner.send_func = Some(inner_send);
    inner.poller_poll_func = Some(inner_poll);
    inner.recvmmsg_func = Some(inner_recvmmsg);
    let mut wrapped: aeron_udp_channel_transport_bindings_stct = unsafe { std::mem::zeroed() };
    unsafe { rusteron_dpdk_stage_wrap_bindings(&inner, &mut wrapped) };

    let frame = data_frame(STREAM, 128, 0, 1);
    let mut iov = [iovec {
        iov_base: frame.as_ptr() as *mut c_void,
        iov_len: frame.len(),
    }];
    let mut bytes = 0i64;
    let rc = unsafe {
        wrapped.send_func.unwrap()(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            iov.as_mut_ptr(),
            1,
            &mut bytes,
        )
    };
    assert_eq!((rc, unsafe { INNER_SENDS }), (1, 1));

    let mut rx: Vec<RxDatagram> = Vec::new();
    let rc = unsafe {
        wrapped.poller_poll_func.unwrap()(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            &mut bytes,
            Some(recv_cb),
            wrapped.recvmmsg_func,
            &mut rx as *mut Vec<RxDatagram> as *mut c_void,
        )
    };
    assert_eq!(rc, 2);
    assert_eq!(rx.len(), 2, "every datagram reaches the driver's callback");
    assert!(
        unsafe { INNER_RECV_SEEN }.flatten().unwrap() as usize != recv_cb as *const () as usize,
        "the inner poll gets the stamping trampoline"
    );
    unsafe { rusteron_dpdk_lab_drain() };
    let (rows, notes) = read(&file);
    assert!(notes.iter().any(|n| n.ends_with(" transport=kernel")), "{notes:?}");
    let m = by_message(&rows);
    let tx = &m[&(STREAM, 0)];
    let send = stage(tx, "tx-send");
    assert_eq!(send.tsc, stage(tx, "tx-call").tsc);
    assert!(send.tsc <= stage(tx, "tx-done").tsc);
    assert!(tx.iter().filter(|r| r.stage.starts_with("rx-")).count() == 3, "{tx:?}");
    let echo = &m[&(ECHO_STREAM, 1)];
    assert_eq!(
        echo.len(),
        3,
        "each stage once: the poll's and recvmmsg's wrappers stamp one layer: {echo:?}"
    );
    let burst = stage(echo, "rx-burst");
    assert_eq!(burst.tsc, stage(echo, "rx-dispatch").tsc);
    assert!(burst.tsc <= stage(echo, "rx-inserted").tsc);

    // off: the inner calls get the driver's own callback, and nothing is stamped
    unsafe { rusteron_dpdk_lab_close() };
    rx.clear();
    unsafe {
        wrapped.poller_poll_func.unwrap()(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            &mut bytes,
            Some(recv_cb),
            wrapped.recvmmsg_func,
            &mut rx as *mut Vec<RxDatagram> as *mut c_void,
        )
    };
    assert_eq!(
        unsafe { INNER_RECV_SEEN }.flatten().unwrap() as usize,
        recv_cb as *const () as usize
    );
    assert_eq!(rx.len(), 2);
}

// ---- capture ----

/// pcapng's blocks: (type, body).
fn blocks(bytes: &[u8]) -> Vec<(u32, Vec<u8>)> {
    let mut out = Vec::new();
    let mut at = 0;
    while at + 12 <= bytes.len() {
        let t = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        let len = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        assert!(len >= 12 && at + len <= bytes.len(), "block at {at}: length {len}");
        let trailer = u32::from_le_bytes(bytes[at + len - 4..at + len].try_into().unwrap()) as usize;
        assert_eq!(trailer, len, "block at {at}: its two lengths");
        out.push((t, bytes[at + 8..at + len - 4].to_vec()));
        at += len;
    }
    assert_eq!(at, bytes.len(), "trailing bytes");
    out
}

/// Every frame the ports receive and send in the window is captured as
/// pcapng: a section header, one interface a port (sender, receiver), then
/// each frame on its port's interface with its direction, its first bytes
/// and its length on the wire.
#[test]
#[serial]
fn the_capture_window_writes_pcapng() {
    let file = path("capture.pcapng");
    let _e = (
        EnvGuard::set("RUSTERON_DPDK_CAPTURE_FILE", &file),
        EnvGuard::set("RUSTERON_DPDK_CAPTURE_WINDOW_MS", "0:600000"),
    );
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.init_rx(RECEIVER_IP, 60000, 1472);
    h.arp_seed(PEER_IP, PEER_MAC);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let big = vec![9u8; 1200];
    assert_eq!(h.send_to((PEER_IP, 40123), &[&data_frame(STREAM, 0, 0, 1)]).0, 1);
    assert_eq!(h.send_to((PEER_IP, 40123), &[&big]).0, 1);
    let f = frame_to_receiver(60000, &data_frame(ECHO_STREAM, 0, 0, 3));
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, f.as_ptr(), f.len(), RX_GOOD, 1) },
        0
    );
    assert_eq!(poll(&mut h, &mut p), 1);
    unsafe { rusteron_dpdk_lab_drain() };
    let bytes = std::fs::read(&file).unwrap();
    let b = blocks(&bytes);
    assert_eq!(b[0].0, 0x0A0D0D0A, "a section header first");
    assert_eq!(u32::from_le_bytes(b[0].1[0..4].try_into().unwrap()), 0x1A2B3C4D);
    assert_eq!((b[1].0, b[2].0), (1, 1), "two interfaces");
    for (i, name) in [(1usize, &b"sender"[..]), (2, &b"receiver"[..])] {
        assert_eq!(u16::from_le_bytes(b[i].1[0..2].try_into().unwrap()), 1, "Ethernet");
        assert_eq!(u32::from_le_bytes(b[i].1[4..8].try_into().unwrap()), 160, "snap length");
        assert!(b[i].1.windows(name.len()).any(|w| w == name), "if_name {name:?}");
    }
    let packets: Vec<&Vec<u8>> = b.iter().filter(|(t, _)| *t == 6).map(|(_, body)| body).collect();
    assert_eq!(packets.len(), 3, "two sent, one received");
    let fields = |body: &Vec<u8>| {
        let w = |i: usize| u32::from_le_bytes(body[i..i + 4].try_into().unwrap());
        let cap = w(12) as usize;
        let padded = (cap + 3) & !3;
        let flags = u32::from_le_bytes(body[20 + padded + 4..20 + padded + 8].try_into().unwrap());
        (w(0), cap, w(16), flags & 3, body[20..20 + cap].to_vec())
    };
    let (iface, cap, len, dir, data) = fields(packets[0]);
    assert_eq!(
        (iface, cap, len, dir),
        (0, 14 + 20 + 8 + 96, 14 + 20 + 8 + 96, 2),
        "the sent message, whole"
    );
    assert_eq!(&data[42..], &data_frame(STREAM, 0, 0, 1)[..]);
    let (iface, cap, len, dir, _) = fields(packets[1]);
    assert_eq!(
        (iface, cap, len, dir),
        (0, 160, 14 + 20 + 8 + 1200, 2),
        "the big one cut at the snap length"
    );
    let (iface, cap, len, dir, data) = fields(packets[2]);
    assert_eq!(
        (iface, cap as usize, len as usize, dir),
        (1, f.len(), f.len(), 1),
        "the received one, inbound"
    );
    assert_eq!(data, f);
    drop(p);
    assert_no_leak();
}

/// Before its window nothing is captured: the hook is off.
#[test]
#[serial]
fn nothing_is_captured_outside_the_window() {
    let file = path("later.pcapng");
    let _e = (
        EnvGuard::set("RUSTERON_DPDK_CAPTURE_FILE", &file),
        EnvGuard::set("RUSTERON_DPDK_CAPTURE_WINDOW_MS", "600000:1000"),
    );
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.arp_seed(PEER_IP, PEER_MAC);
    assert_eq!(h.send_to((PEER_IP, 40123), &[&data_frame(STREAM, 0, 0, 1)]).0, 1);
    unsafe { rusteron_dpdk_lab_drain() };
    let bytes = std::fs::read(&file).unwrap();
    assert!(blocks(&bytes).iter().all(|(t, _)| *t != 6), "no packet");
    assert_no_leak();
}

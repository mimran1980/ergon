//! End-to-end: rustls server → our handshake → in-place record layer →
//! WebSocket parser, through the driver. Covers a frame in the same record as
//! the 101, a frame spanning several TLS records, fragmentation and ping/pong.
#![cfg(target_os = "linux")]
#![allow(missing_docs, clippy::cast_possible_truncation)]

mod support;

use ergon_io::{Backend, Config, ConnId, Driver, Handler, RxMeta, WsConn, WsHandler, ws};
use support::{Lab, R};

#[derive(Default)]
struct Collect(Vec<(u8, Vec<u8>)>);

impl Handler for Collect {}

impl WsHandler for Collect {
    fn on_message(&mut self, _: ConnId, opcode: u8, payload: &[u8], _: &RxMeta, _: u16) {
        self.0.push((opcode, payload.to_vec()));
    }
}

#[test]
fn full_stack_over_loopback() -> R {
    let lab = Lab::new()?;
    let target = lab.target("/x?y=1")?;
    let big: Vec<u8> = (0..40_000u32).map(|i| (i % 253) as u8).collect();
    let big2 = big.clone();
    // The 40 kB frame goes out once the client's driver has delivered
    // "hello": sent sooner, its records can arrive with the 101 and be read
    // by the upgrade, which hands them over as one plaintext block, so they
    // never span records in the driver.
    let (upgraded, wait_upgraded) = std::sync::mpsc::channel::<()>();
    std::thread::scope(|s| -> R {
        let lab = &lab;
        let h = s.spawn(move || -> R {
            let mut p = lab.accept()?;
            assert_eq!(p.path, "/x?y=1");
            p.send(ws::OP_TEXT, true, b"hello")?;
            wait_upgraded.recv()?;
            p.send(ws::OP_BINARY, true, &big2)?;
            p.send(ws::OP_TEXT, false, b"ab")?;
            p.send(ws::OP_CONT, true, b"cd")?;
            p.send(ws::OP_PING, true, b"p1")?;
            assert_eq!(p.recv()?, (ws::OP_PONG, b"p1".to_vec()));
            p.send(ws::OP_TEXT, true, b"bye")?;
            Ok(())
        });
        let mut d: Driver<WsConn> = Driver::new(Config::new(Backend::EpollSpin))?;
        let id = d.add(WsConn::connect(&target, lab.client_cfg.clone(), 1 << 20)?)?;
        let mut got = Collect::default();
        while got.0.is_empty() {
            d.poll(&mut got)?;
        }
        upgraded.send(())?;
        while got.0.len() < 4 {
            d.poll(&mut got)?;
        }
        h.join().map_err(|_| "server panicked")??;
        let conn = d.get(id).ok_or("closed")?;
        assert_eq!(got.0[0], (ws::OP_TEXT, b"hello".to_vec()));
        assert_eq!(got.0[1], (ws::OP_BINARY, big));
        assert_eq!(got.0[2], (ws::OP_TEXT, b"abcd".to_vec()));
        assert_eq!(got.0[3], (ws::OP_TEXT, b"bye".to_vec()));
        assert_eq!(conn.stats().pings, 1);
        assert!(
            conn.stats().span_copy_bytes > 0,
            "the 40 kB frame spans records"
        );
        assert_eq!(conn.stats().frag_copy_bytes, 4);
        Ok(())
    })
}

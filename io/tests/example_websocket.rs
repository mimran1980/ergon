//! Runs `examples/websocket.rs` against the local test server, on every
//! backend: subscribe, messages, a server close, the reconnect and second
//! subscribe, then the clean close.
#![cfg(target_os = "linux")]
#![allow(missing_docs)]

mod support;

// The example's `main` is not called here. (Without `webpki-roots` the
// example allows dead code itself; a second allow would be a duplicate.)
#[path = "../examples/websocket.rs"]
#[cfg_attr(feature = "webpki-roots", allow(dead_code))]
mod example;

use std::time::Duration;

use ergon_io::placement::HelperThread;
use ergon_io::reconnect::{LinkState, Retry};
use ergon_io::{Backend, Config, WsDial, WsOptions, ws};
use support::{Lab, R};

fn backend(b: Backend) -> R {
    let lab = Lab::new()?;
    let target = lab.target("/ws")?;
    let tls = lab.client_cfg.clone();
    // Fast retries; a broken run gives up instead of hanging.
    let retry = Retry {
        initial_ms: 10,
        max_ms: 50,
        max_attempts: 5,
        ..Retry::default()
    };
    let dial = move || WsDial::with_options(target.clone(), tls.clone(), WsOptions::default());
    let mut app = example::App::new(retry, dial, 3)?;
    let mut cfg = Config::new(b);
    cfg.wait_us = cfg.wait_us.min(100);
    cfg.connect_timeout_ms = 2_000;
    // A pod with one CPU has no core for the connect helper the example's
    // reconnect uses: opt in to opening on the loop there, so the example's
    // lifecycle runs on any pod (the refusal without this is reconnect.rs's
    // `no_cpu_for_the_connect_helper_gives_up_at_once`). On a pod with a
    // second CPU the fallback is off: the run exercises the helper path.
    let one_cpu = ergon_io::placement::process_cpus(&ergon_tune::Host::system())
        .0
        .len()
        <= 1;
    cfg.inline_connect_fallback = one_cpu;
    let subscribe = (ws::OP_TEXT, example::SUBSCRIBE.as_bytes().to_vec());
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            // `uring-sqpoll` on a pod with one CPU refuses at `Driver::new`,
            // before any dial: the server would otherwise wait out the full
            // spell for a client that never comes.
            let wait = if one_cpu && b == Backend::UringSqpoll {
                Duration::from_millis(200)
            } else {
                Duration::from_secs(15)
            };
            // First connection: two messages, then the server restarts.
            let mut p = lab.accept_within(wait)?;
            assert_eq!(p.recv()?, subscribe);
            p.send(ws::OP_TEXT, true, b"one")?;
            p.send(ws::OP_TEXT, true, b"two")?;
            p.send_close(Some(1012), "restart")?;
            assert_eq!(p.recv()?, (ws::OP_CLOSE, 1012u16.to_be_bytes().to_vec()));
            drop(p);
            // The reconnect subscribes again; the third message ends the run.
            let mut p = lab.accept_within(wait)?;
            assert_eq!(p.recv()?, subscribe);
            p.send(ws::OP_TEXT, true, b"three")?;
            let mut close = 1000u16.to_be_bytes().to_vec();
            close.extend_from_slice(b"done");
            assert_eq!(p.recv()?, (ws::OP_CLOSE, close));
            p.send_close(Some(1000), "")?;
            assert!(
                p.client_closed()?,
                "the client closes TCP after the handshake"
            );
            Ok(())
        });
        let ran = example::run(cfg, &mut app);
        let srv = srv.join().map_err(|_| "server panicked")?;
        // Prefer the client's error over the server's: on a pod with one
        // CPU `Driver::new` refuses `uring-sqpoll` typed before any dial,
        // and the server would only report "no client connected".
        match ran {
            Err(e) => Err(Box::new(e)),
            Ok(()) => srv,
        }
    })?;
    assert_eq!((app.messages, app.opens), (3, 2));
    assert_eq!(app.links.state(app.link), Some(LinkState::Stopped));
    Ok(())
}

#[test]
fn the_example_subscribes_reconnects_and_closes_cleanly() -> R {
    let mut ran = 0;
    for b in Backend::ALL {
        match backend(b) {
            Ok(()) => ran += 1,
            // A pod with one CPU has no core for `uring-sqpoll`'s kernel
            // thread: `Driver::new` refuses typed (asserted; covered in full
            // by driver.rs's `sqpoll_thread_never_shares_the_polling_cpu`).
            // The body still runs on every other backend.
            Err(e) if b == Backend::UringSqpoll && support::refused(&*e, HelperThread::SqPoll) => {}
            Err(e) => return Err(format!("{b}: {e}").into()),
        }
    }
    assert!(ran >= Backend::ALL.len() - 1, "only {ran} backends ran");
    Ok(())
}

//! A complete WebSocket client: connect, subscribe, print each message,
//! reconnect and subscribe again after a drop, then close cleanly.
//!
//! - Linux only (the `Driver` is Linux-only).
//! - Run: `cargo run -p ergon-io --features webpki-roots --example websocket -- 20 recv-busy`
//! - Arguments: messages to print before closing (default 20), then the
//!   backend configuration (default `recv-busy`, the pick for a few sockets).
//! - The feed is Binance's public spot stream; any public feed will do.
//! - `io/tests/example_websocket.rs` runs this file against a local server,
//!   on every backend.
#![cfg_attr(
    not(all(target_os = "linux", feature = "webpki-roots")),
    allow(dead_code)
)]

// ANCHOR: example
use ergon_io::reconnect::{Closed, LinkId, LinkState, Retry};
use ergon_io::{ConnId, Error, Handler, Reconnect, RxMeta, WsDial, WsHandler};

/// Sent on every connect and reconnect.
pub const SUBSCRIBE: &str = r#"{"method":"SUBSCRIBE","params":["btcusdt@bookTicker"],"id":1}"#;

/// The application. `Driver::poll` calls it.
pub struct App {
    /// Keeps the connection open: reopens it with backoff after a drop.
    pub links: Reconnect<WsDial>,
    /// The one link (a connection that `links` keeps open).
    pub link: LinkId,
    /// Connections opened during the last poll: subscribe on them.
    opened: Vec<ConnId>,
    /// Close after this many messages.
    pub limit: u64,
    /// Messages received.
    pub messages: u64,
    /// Connections opened; more than 1 means it reconnected.
    pub opens: u32,
}

impl App {
    /// `dial` builds a fresh connection set-up for each attempt.
    ///
    /// # Errors
    /// `retry` is out of range.
    pub fn new(
        retry: Retry,
        dial: impl FnMut() -> ergon_io::Result<WsDial> + 'static,
        limit: u64,
    ) -> ergon_io::Result<Self> {
        let mut links = Reconnect::new(retry)?;
        let link = links.add(dial);
        Ok(Self {
            links,
            link,
            opened: Vec::new(),
            limit,
            messages: 0,
            opens: 0,
        })
    }

    /// Closed cleanly, or the link gave up.
    #[must_use]
    pub fn finished(&self) -> bool {
        matches!(
            self.links.state(self.link),
            Some(LinkState::Stopped | LinkState::GaveUp)
        )
    }
}

impl Handler for App {
    fn on_open(&mut self, id: ConnId) {
        if let Some(opened) = self.links.on_open(id) {
            self.opens += 1;
            if opened.reconnect {
                println!("reconnected: subscribing again");
            }
            // Sends wait until `poll` returns: remember the connection.
            self.opened.push(id);
        }
    }

    fn on_closed(&mut self, id: ConnId, error: Error) {
        println!("closed: {error}");
        if let Some(Closed::Retry { delay_ms, .. }) = self.links.on_closed(id) {
            println!("reconnecting in {delay_ms} ms");
        }
    }
}

impl WsHandler for App {
    fn on_message(&mut self, _: ConnId, _: u8, payload: &[u8], _: &RxMeta, _: u16) {
        // `payload` points into the receive buffer: use it during this call only.
        println!("{}", std::str::from_utf8(payload).unwrap_or("<binary>"));
        self.messages += 1;
    }
}

/// The loop: runs until the clean close, or until the link gives up.
///
/// # Errors
/// The backend failed. A connection that fails goes to `on_closed` instead.
#[cfg(target_os = "linux")]
pub fn run(config: ergon_io::Config, app: &mut App) -> ergon_io::Result<()> {
    use ergon_io::{Driver, WsConn};

    let mut driver: Driver<WsConn> = Driver::new(config)?;
    let mut closing = false;
    while !app.finished() {
        driver.poll(app)?; // receive, decrypt, deliver, send
        app.links.poll(&mut driver); // start a reconnect when one is due
        for id in app.opened.drain(..) {
            // A connection the server is already closing takes no sends.
            if let Some(mut conn) = driver.get_mut(id)
                && !conn.is_closing()
            {
                conn.send_text(SUBSCRIBE)?;
            }
        }
        if app.messages >= app.limit && !closing {
            closing = true;
            app.links.stop(app.link); // no reconnect after this close
            if let Some(LinkState::Open(id)) = app.links.state(app.link)
                && let Some(mut conn) = driver.get_mut(id)
                && !conn.is_closing()
            {
                // `on_closed` follows once the server answers.
                conn.close(1000, "done")?;
            }
        }
    }
    Ok(())
}

#[cfg(all(target_os = "linux", feature = "webpki-roots"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use ergon_io::{Config, Target, WsOptions, tls};

    let mut args = std::env::args().skip(1);
    let limit: u64 = args.next().map_or(Ok(20), |a| a.parse())?;
    let config = Config::parse(&args.next().unwrap_or_else(|| "recv-busy".into()))?;
    // In production, pin this thread to an isolated core first.
    let tls = tls::client_config(tls::webpki_roots())?;
    let target = Target::new("stream.binance.com", 9443, "/ws");
    // Limits, pings and timeouts: WsOptions.
    let dial = move || WsDial::with_options(target.clone(), tls.clone(), WsOptions::default());
    let mut app = App::new(Retry::default(), dial, limit)?;
    run(config, &mut app)?;
    println!("{} messages, {} connection(s)", app.messages, app.opens);
    Ok(())
}
// ANCHOR_END: example

#[cfg(not(all(target_os = "linux", feature = "webpki-roots")))]
fn main() {
    eprintln!("needs Linux: cargo run -p ergon-io --features webpki-roots --example websocket");
}

//! The production Invoker writes the journal; the engine replays those exact rows.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

use engine::{agent::Engine, replay};
use ergon_runtime::Settings;
use ergon_runtime::bus::Bus;
use ergon_runtime::clock::Clock;
use ergon_runtime::frames;
use ergon_runtime::journal::{Input, InputEvent, Journal};
use ergon_runtime::persist::Persist;
use ergon_runtime::rt::sim::{Sim, SimConfig};
use ergon_runtime::rt::{Agent, Config, Ctx, Expiry, FeedId, Invoker};
use ergon_runtime::source::SOURCE_TEMPLATE_ID;
use ergon_runtime::subscription::Delivery;
use lab::Streams;
use rusteron_media_driver::bindings::aeron_threading_mode_t;
use rusteron_media_driver::{AeronDriver, AeronDriverContext};
use schema::trading::AnyMessage;

const IPC: &str = "aeron:ipc";

/// Raw frames by publication (session, stream), each with its position.
type Publications = BTreeMap<(i32, i32), Vec<(i64, Vec<u8>)>>;
const RAW: [i32; 2] = [9611, 9612];

struct Adapter {
    engine: Engine,
    raw: Vec<FeedId>,
    frames: Vec<Vec<u8>>,
}

impl Agent for Adapter {
    fn start(&mut self, ctx: &mut Ctx) -> Result<(), ergon_runtime::Error> {
        self.engine.start(ctx)?;
        self.raw = RAW
            .iter()
            .map(|&stream| ctx.subscribe_channel(IPC, stream))
            .collect();
        Ok(())
    }
    fn on_message(&mut self, ctx: &mut Ctx, feed: FeedId, frame: &[u8], delivery: Delivery) {
        if let Some(venue) = self.raw.iter().position(|&id| id == feed) {
            self.frames.push(frame.to_vec());
            self.engine
                .on_message(ctx, FeedId(2 + 2 * venue as u32), frame, delivery);
        } else {
            self.engine.on_message(ctx, feed, frame, delivery);
        }
    }
    fn on_timer(&mut self, ctx: &mut Ctx, timer: Expiry) {
        self.engine.on_timer(ctx, timer);
    }
    fn stop(&mut self, ctx: &mut Ctx) {
        self.engine.stop(ctx);
    }
}

fn is_output(frame: &[u8]) -> bool {
    matches!(
        AnyMessage::decode(frame, 0),
        Ok(AnyMessage::Ema(_) | AnyMessage::AggBook(_) | AnyMessage::NewOrder(_))
    )
}

#[test]
fn production_live_journal_replays_engine_frames() -> Result<(), Box<dyn std::error::Error>> {
    let directory =
        std::env::temp_dir().join(format!("engine-live-journal-{}", std::process::id()));
    std::fs::create_dir_all(&directory)?;
    let driver_path = directory.join("driver");
    let context = AeronDriverContext::new()?;
    context.set_dir(&rusteron_media_driver::cformat!(
        "{}",
        driver_path.display()
    ))?;
    context.set_dir_delete_on_start(true)?;
    context.set_dir_delete_on_shutdown(true)?;
    context.set_threading_mode(aeron_threading_mode_t::AERON_THREADING_MODE_SHARED)?;
    let _driver = AeronDriver::launch_embedded_guard(context, false);
    let tables = directory.join("tables.yaml");
    std::fs::write(
        &tables,
        "tables:\n  ema: {kind: dynamic}\n  agg_book: {kind: dynamic}\n  new_order: {kind: dynamic}\n",
    )?;
    let settings = Settings {
        aeron_dir: Some(driver_path.to_string_lossy().into_owned()),
        channel: IPC.into(),
        stream_id: 9600,
        subscriber_timeout: Duration::ZERO,
        ..Settings::new(tables)
    };
    let bus = Bus::connect(&settings)?;
    let persist = Persist::connect(schema::TRADING_SCHEMA, &bus, settings)?;
    let mut inputs = bus.subscription(IPC, 9600);
    let streams = Streams::parse(replay::STREAMS)?;
    let mut signals = bus.subscription(&streams.spy("engine-r1", "127.0.0.1")?, 105);
    let mut orders = bus.subscription(&streams.spy("engine-r1", "127.0.0.1")?, 106);
    let clock = Clock::new();
    let deadline = clock.now().0 + 15_000_000_000;
    while !persist.is_connected() || !inputs.is_connected() {
        let _ = bus.poll();
        inputs.poll(|_, _| {}, 256);
        assert!(clock.now().0 < deadline, "Persist journal did not connect");
        std::thread::sleep(Duration::from_millis(1));
    }
    let mut runtime = Invoker::new(Config {
        persist: Some(persist),
        region: replay::REGION.into(),
        journal: true,
        directory: Box::new(streams.clone()),
        ..Config::new(bus)
    })?;
    let engine = Engine::new(runtime.ctx(), streams)?;
    let mut adapter = Adapter {
        engine,
        raw: Vec::new(),
        frames: Vec::new(),
    };
    runtime.start(&mut adapter)?;
    let publishers = RAW
        .map(|stream| runtime.ctx().publish_channel(IPC, stream))
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let mut journal = Journal::default();
    let mut live_signals = Vec::new();
    let mut live_orders = Vec::new();
    let market = replay::inbound(&replay::Market::FIXTURE);
    let mut records = frames::parse(&market)?.records.peekable();
    let mut sent = 0;
    // Whether the first journal row came after persist's `Source` message,
    // which names the run the ingester stores it under.
    let (mut sourced, mut first_row_sourced) = (false, None);
    loop {
        runtime.cycle(&mut adapter);
        inputs.poll(
            |frame, _| {
                let id = |at: usize| {
                    frame
                        .get(at..at + 2)
                        .map(|b| u16::from_le_bytes([b[0], b[1]]))
                };
                sourced |= id(2) == Some(SOURCE_TEMPLATE_ID)
                    && id(4) == Some(ergon_runtime::event::SCHEMA_ID);
                if let Ok(input) = Input::decode(frame) {
                    first_row_sourced.get_or_insert(sourced);
                    journal.inputs.push(input);
                }
            },
            1024,
        );
        signals.poll(
            |frame, _| {
                if is_output(frame) {
                    live_signals.push(frame.to_vec());
                }
            },
            1024,
        );
        orders.poll(
            |frame, _| {
                if is_output(frame) {
                    live_orders.push(frame.to_vec());
                }
            },
            1024,
        );
        // The engine's publications connect through these spies (`ssc=true`):
        // an order sent before they do is dropped live, not in the replay.
        if signals.is_connected()
            && orders.is_connected()
            && publishers
                .iter()
                .all(|&out| runtime.ctx_ref().is_connected(out))
            && let Some(record) = records.next()
        {
            let template = u16::from_le_bytes([record.frame[2], record.frame[3]]);
            runtime.ctx_ref().send(
                publishers[record.stream as usize],
                template,
                record.frame.len(),
                |buf| {
                    buf.copy_from_slice(record.frame);
                    Ok::<_, std::convert::Infallible>(buf.len())
                },
            )?;
            sent += 1;
        }
        let dispatched = adapter
            .frames
            .iter()
            .filter(|frame| frame.get(4..6) == Some(&schema::market::SCHEMA_ID.to_le_bytes()[..]))
            .count();
        if records.peek().is_none() && dispatched == sent && !live_signals.is_empty() {
            break;
        }
        assert!(
            clock.now().0 < deadline,
            "live engine did not consume the market and fire its timer: {} of {} records sent, {dispatched} dispatched, {} signals",
            sent,
            sent + records.clone().count(),
            live_signals.len()
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    runtime.finish(&mut adapter)?;
    // The journal's last frames are in the log already: reading them needs
    // no conductor, and the runtime that owns the client has finished.
    while !journal
        .inputs
        .iter()
        .any(|input| matches!(input.event, InputEvent::Stop))
    {
        inputs.poll(
            |frame, _| {
                if let Ok(input) = Input::decode(frame) {
                    journal.inputs.push(input);
                }
            },
            1024,
        );
        signals.poll(
            |frame, _| {
                if is_output(frame) {
                    live_signals.push(frame.to_vec());
                }
            },
            1024,
        );
        orders.poll(
            |frame, _| {
                if is_output(frame) {
                    live_orders.push(frame.to_vec());
                }
            },
            1024,
        );
        assert!(clock.now().0 < deadline, "live stop checkpoint was lost");
    }
    assert_eq!(
        first_row_sourced,
        Some(true),
        "the journal's first row went out before the Source message"
    );
    assert!(
        journal
            .inputs
            .iter()
            .any(|input| matches!(input.event, InputEvent::TimerBatchStart { .. }))
    );
    assert!(
        !live_orders.is_empty(),
        "the market must exercise epoch-seeded live order ids"
    );
    // The raw-frame table as ClickHouse would hold it: each publication's
    // frames by position.
    let mut table = Publications::new();
    for ((position, session, stream), frame) in journal
        .inputs
        .iter()
        .filter_map(|input| match input.event {
            InputEvent::Message {
                position,
                session,
                stream,
                ..
            } => Some((position, session, stream)),
            _ => None,
        })
        .zip(adapter.frames)
    {
        table
            .entry((session, stream))
            .or_default()
            .push((position, frame));
    }
    // One query per publication, over its positions: a query per message
    // would leave a connection unanswered, and too few would time out here.
    let publications = table.len();
    #[expect(
        clippy::disallowed_methods,
        reason = "fake ClickHouse peer: a literal loopback address, nothing to resolve"
    )]
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let address = listener.local_addr()?;
    #[expect(
        clippy::disallowed_methods,
        reason = "fake ClickHouse peer: the blocking replay under test waits on it"
    )]
    let server = std::thread::spawn(move || -> Result<(), String> {
        let clock = Clock::new();
        for _ in 0..publications {
            let deadline = clock.now().0 + 30_000_000_000;
            let mut socket = loop {
                match listener.accept() {
                    Ok((socket, _)) => break socket,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        if clock.now().0 > deadline {
                            return Err("the replay asked for fewer publications".into());
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(e) => return Err(e.to_string()),
                }
            };
            socket.set_nonblocking(false).map_err(|e| e.to_string())?;
            socket
                .set_read_timeout(Some(Duration::from_secs(10)))
                .map_err(|e| e.to_string())?;
            let mut header = Vec::new();
            while !header.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                socket.read_exact(&mut byte).map_err(|e| e.to_string())?;
                header.push(byte[0]);
            }
            let header = String::from_utf8(header).map_err(|e| e.to_string())?;
            let length = header
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|n| n.trim().parse::<usize>())
                })
                .ok_or("missing HTTP length")?
                .map_err(|e| e.to_string())?;
            let mut sql = vec![0; length];
            socket.read_exact(&mut sql).map_err(|e| e.to_string())?;
            let sql = String::from_utf8(sql).map_err(|e| e.to_string())?;
            let number = |after: &str| -> Result<i64, String> {
                sql.split(after)
                    .nth(1)
                    .and_then(|rest| rest.split_whitespace().next())
                    .and_then(|n| n.parse().ok())
                    .ok_or_else(|| format!("no {after:?} in {sql}"))
            };
            let session = i32::try_from(number("session_id = ")?).map_err(|e| e.to_string())?;
            let stream = i32::try_from(number("stream_id = ")?).map_err(|e| e.to_string())?;
            let mut bounds = sql
                .split_once("position BETWEEN ")
                .map(|(_, range)| range)
                .unwrap_or_default()
                .split(" AND ")
                .map(|n| {
                    n.split_whitespace()
                        .next()
                        .and_then(|n| n.parse::<i64>().ok())
                });
            let (Some(Some(low)), Some(Some(high))) = (bounds.next(), bounds.next()) else {
                return Err(format!("no position range in {sql}"));
            };
            let mut body = Vec::new();
            for (position, frame) in table
                .get(&(session, stream))
                .ok_or_else(|| format!("no publication {session}/{stream}: {sql}"))?
                .iter()
                .filter(|(position, _)| (low..=high).contains(position))
            {
                body.extend_from_slice(&position.to_le_bytes());
                let mut len = frame.len() as u64;
                while len >= 128 {
                    body.push((len as u8 & 127) | 128);
                    len >>= 7;
                }
                body.push(len as u8);
                body.extend_from_slice(frame);
            }
            write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .map_err(|e| e.to_string())?;
            socket.write_all(&body).map_err(|e| e.to_string())?;
        }
        Ok(())
    });
    let mut config = SimConfig::new();
    config.region = replay::REGION.into();
    config.from = journal.inputs.first().map(|input| input.ts);
    config.journal = Some(journal);
    config.clickhouse = Some(ergon_runtime::clickhouse_source::ClickHouseConfig {
        client: ergon_runtime::clickhouse::ClickHouse::new(
            &format!("http://{address}"),
            "default",
            "",
            "test",
        ),
        table: "frame".into(),
    });
    let mut exact = Sim::new(config, Vec::new())?;
    let engine = Engine::new(exact.ctx(), Streams::parse(replay::STREAMS)?)?;
    let mut adapter = Adapter {
        engine,
        raw: Vec::new(),
        frames: Vec::new(),
    };
    exact.run(&mut adapter)?;
    server
        .join()
        .map_err(|_| "raw-frame fixture server panicked")?
        .map_err(std::io::Error::other)?;
    let outputs = exact.ctx().captured();
    let replayed_signals: Vec<_> = outputs
        .records()
        .filter(|r| {
            outputs
                .names()
                .get(r.stream as usize)
                .is_some_and(|name| name == "engine-r1/signals")
        })
        .map(|r| r.frame.to_vec())
        .collect();
    let replayed_orders: Vec<_> = outputs
        .records()
        .filter(|r| {
            outputs
                .names()
                .get(r.stream as usize)
                .is_some_and(|name| name == "engine-r1/orders")
        })
        .map(|r| r.frame.to_vec())
        .collect();
    assert_eq!(replayed_signals, live_signals);
    assert_eq!(replayed_orders, live_orders);
    Ok(())
}

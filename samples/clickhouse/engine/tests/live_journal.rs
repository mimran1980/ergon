//! The production Invoker writes the journal; the engine replays those exact rows.
#![cfg(feature = "clickhouse")]

use std::ffi::CString;
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
use ergon_runtime::streams::Streams;
use ergon_runtime::subscription::Delivery;
use rusteron_media_driver::{AeronDriver, AeronDriverContext, aeron_threading_mode_t};
use schema::trading::AnyMessage;

const IPC: &str = "aeron:ipc";
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
    context.set_dir(&CString::new(driver_path.to_string_lossy().as_bytes())?)?;
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
        aeron_invoker: true,
        exclusive: true,
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
        let _ = bus.do_work();
        inputs.poll(|_, _| {}, 256);
        assert!(clock.now().0 < deadline, "Persist journal did not connect");
        std::thread::sleep(Duration::from_millis(1));
    }
    let mut runtime = Invoker::new(Config {
        persist: Some(persist),
        region: replay::REGION.into(),
        journal: true,
        ..Config::new(bus.clone(), streams)
    })?;
    let engine = Engine::new(runtime.ctx())?;
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
    loop {
        runtime.cycle(&mut adapter);
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
        if publishers
            .iter()
            .all(|&out| runtime.ctx_ref().is_connected(out))
        {
            if let Some(record) = records.next() {
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
            "live engine did not consume the market and fire its timer"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    runtime.finish(&mut adapter)?;
    while !journal
        .inputs
        .iter()
        .any(|input| matches!(input.event, InputEvent::Stop))
    {
        let _ = bus.do_work();
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
    let references: Vec<_> = journal
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
        .collect();
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?;
    let server = std::thread::spawn(move || -> Result<(), String> {
        for ((position, session, stream), frame) in references {
            let (mut socket, _) = listener.accept().map_err(|e| e.to_string())?;
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
            if !sql.contains(&format!("position = {position}"))
                || !sql.contains(&format!("session_id = {session}"))
                || !sql.contains(&format!("stream_id = {stream}"))
            {
                return Err(format!("incorrect raw-frame locator: {sql}"));
            }
            let mut body = Vec::new();
            let mut len = frame.len() as u64;
            while len >= 128 {
                body.push((len as u8 & 127) | 128);
                len >>= 7;
            }
            body.push(len as u8);
            body.extend(frame);
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
    let mut config = SimConfig::new(Streams::parse(replay::STREAMS)?);
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
    let engine = Engine::new(exact.ctx())?;
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

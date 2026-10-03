//! Exact engine dispatch replay, including seeded live-style order ids and timers.
use engine::{agent::Engine, replay};
use ergon_runtime::frames;
use ergon_runtime::journal::{Input, InputEvent, Journal};
use ergon_runtime::rt::sim::{Sim, SimConfig};
use ergon_runtime::rt::{Agent, Ctx, Expiry, FeedId};
use ergon_runtime::streams::Streams;
use ergon_runtime::subscription::Delivery;
use schema::trading::AnyMessage;

const SEED: u64 = 10_000;

struct Recorded {
    engine: Engine,
    refs: Vec<(Vec<u8>, i64)>,
    journal: Journal,
    next_id: u64,
}

impl Recorded {
    fn checkpoint(&mut self, ctx: &Ctx, event: InputEvent) {
        self.journal.inputs.push(Input {
            sequence: self.journal.inputs.len() as u64,
            wall_offset: 0,
            next_id: self.next_id,
            ts: ctx.now(),
            event,
        });
    }
    fn update_ids(&mut self, ctx: &Ctx) {
        self.next_id = ctx
            .captured()
            .records()
            .filter_map(|r| match AnyMessage::decode(r.frame, 0) {
                Ok(AnyMessage::NewOrder(order)) => Some(order.order_id()),
                _ => None,
            })
            .max()
            .unwrap_or(SEED);
    }
}

impl Agent for Recorded {
    fn start(&mut self, ctx: &mut Ctx) -> Result<(), ergon_runtime::Error> {
        for _ in 0..SEED {
            ctx.next_id();
        }
        self.next_id = SEED;
        self.checkpoint(ctx, InputEvent::Start);
        self.engine.start(ctx)
    }
    fn on_message(&mut self, ctx: &mut Ctx, feed: FeedId, frame: &[u8], delivery: Delivery) {
        let Some((_, position)) = self.refs.iter().find(|(bytes, _)| bytes == frame) else {
            return;
        };
        self.checkpoint(
            ctx,
            InputEvent::Message {
                feed,
                recording: 0,
                position: *position,
                session: 123,
                stream: feed.0 as i32,
                delivery,
            },
        );
        self.engine.on_message(ctx, feed, frame, delivery);
        self.update_ids(ctx);
    }
    fn on_timer(&mut self, ctx: &mut Ctx, timer: Expiry) {
        self.checkpoint(
            ctx,
            InputEvent::Timer {
                token: timer.token,
                deadline: timer.deadline,
                missed: timer.missed,
            },
        );
        self.engine.on_timer(ctx, timer);
        self.update_ids(ctx);
    }
}

#[test]
fn journal_replays_engine_outputs_with_recorded_ids_and_receive_times()
-> Result<(), Box<dyn std::error::Error>> {
    let data = replay::inbound(&replay::Market::FIXTURE);
    let refs = frames::parse(&data)?
        .records
        .map(|r| (r.frame.to_vec(), r.offset as i64))
        .collect();
    let mut config = SimConfig::new(Streams::parse(replay::STREAMS)?);
    config.region = replay::REGION.into();
    config.route_delays.insert("md-alpha/md".into(), 2_000_000);
    config.route_delays.insert("md-beta/md".into(), 3_000_000);
    let mut original = Sim::new(config, vec![data.clone()])?;
    let engine = Engine::new(original.ctx())?;
    let mut recorded = Recorded {
        engine,
        refs,
        journal: Journal::default(),
        next_id: SEED,
    };
    original.run(&mut recorded)?;
    let expected = original.ctx().captured().to_bytes();
    let orders = frames::parse(&expected)?
        .records
        .filter_map(|r| match AnyMessage::decode(r.frame, 0) {
            Ok(AnyMessage::NewOrder(order)) => Some(order.order_id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        orders.iter().any(|id| *id > SEED),
        "fixture must exercise seeded live-style ids"
    );
    assert!(
        recorded
            .journal
            .inputs
            .iter()
            .any(|input| matches!(input.event, InputEvent::Timer { .. })),
        "fixture must exercise actual timer dispatches"
    );
    let mut config = SimConfig::new(Streams::parse(replay::STREAMS)?);
    config.region = replay::REGION.into();
    config.journal = Some(recorded.journal);
    let mut exact = Sim::new(config, vec![data])?;
    let mut engine = Engine::new(exact.ctx())?;
    exact.run(&mut engine)?;
    assert_eq!(exact.ctx().captured().to_bytes(), expected);
    Ok(())
}

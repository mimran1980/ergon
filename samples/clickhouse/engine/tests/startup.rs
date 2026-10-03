//! Agent startup, which can occur after construction crossed a second boundary.
use engine::agent::Engine;
use engine::replay;
use ergon_runtime::clock::Nanos;
use ergon_runtime::rt::sim::{Sim, SimConfig};
use ergon_runtime::rt::{Agent, Ctx, Expiry, FeedId};
use ergon_runtime::streams::Streams;
use ergon_runtime::subscription::Delivery;

struct DeferredStart {
    engine: Engine,
    started: bool,
    premature: Vec<Nanos>,
}

impl Agent for DeferredStart {
    fn start(&mut self, _ctx: &mut Ctx) -> Result<(), ergon_runtime::Error> {
        Ok(())
    }
    fn on_message(&mut self, ctx: &mut Ctx, feed: FeedId, frame: &[u8], delivery: Delivery) {
        if !self.started {
            self.engine
                .start(ctx)
                .unwrap_or_else(|error| panic!("engine start: {error}"));
            self.started = true;
        }
        self.engine.on_message(ctx, feed, frame, delivery);
    }
    fn on_timer(&mut self, ctx: &mut Ctx, expiry: Expiry) {
        if self.started {
            self.engine.on_timer(ctx, expiry);
        } else {
            self.premature.push(ctx.now());
        }
    }
}

#[test]
fn constructor_does_not_arm_timers_before_the_start_checkpoint()
-> Result<(), Box<dyn std::error::Error>> {
    let input = replay::inbound(&replay::Market::FIXTURE);
    let first = ergon_runtime::frames::parse(&input)?
        .records
        .next()
        .ok_or("empty market")?
        .ts;
    let config = SimConfig {
        region: replay::REGION.into(),
        from: Some(Nanos(first.0 - 3_000_000_000)),
        ..SimConfig::new(Streams::parse(replay::STREAMS)?)
    };
    let mut sim = Sim::new(config, vec![input])?;
    let engine = Engine::new(sim.ctx())?;
    let mut deferred = DeferredStart {
        engine,
        started: false,
        premature: Vec::new(),
    };
    sim.run(&mut deferred)?;
    assert!(
        deferred.premature.is_empty(),
        "constructor armed timers before start: {:?}",
        deferred.premature
    );
    assert!(sim.ctx().captured().records().count() > 0);
    Ok(())
}

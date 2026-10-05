//! Engine and a simulated venue driven on the same thread and clock.
use ergon_runtime::Error;

/// Run configured historical sources with a composite engine and venue,
/// both reading the lab's registry `streams`.
///
/// # Errors
/// A source, stream, or agent cannot be initialized.
pub fn execute(
    config: ergon_runtime::rt::sim::SimConfig,
    streams: lab::Streams,
    logs: Vec<Vec<u8>>,
) -> Result<Vec<u8>, Error> {
    let mut sim = ergon_runtime::rt::sim::Sim::new(config, logs)?;
    let engine = crate::agent::Engine::new(sim.ctx(), streams.clone())?;
    let venue = crate::venue::Venue::new(sim.ctx(), &streams)?;
    sim.run(&mut (engine, venue))?;
    Ok(sim.ctx().captured().to_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use schema::trading::{AnyMessage, OrderStatus};

    /// A recorded market through the engine and the simulated venue, each
    /// way `latency_ns` apart.
    fn run(input: Vec<u8>, latency_ns: i64) -> Result<Vec<u8>, Error> {
        let mut config = ergon_runtime::rt::sim::SimConfig::new();
        config.region = crate::replay::REGION.into();
        config
            .loopback
            .insert(format!("engine-{}/orders", config.region), latency_ns);
        config
            .loopback
            .insert(format!("exch-sim-{}/exec", config.region), latency_ns);
        execute(
            config,
            lab::Streams::parse(crate::replay::STREAMS)?,
            vec![input],
        )
    }

    #[test]
    fn composite_backtest_trades_and_is_deterministic() -> Result<(), Box<dyn std::error::Error>> {
        let input = crate::replay::inbound(&crate::replay::Market::FIXTURE);
        let first = run(input.clone(), 100_000)?;
        let second = run(input, 100_000)?;
        assert_eq!(first, second);
        let frames = ergon_runtime::frames::parse(&first)?;
        let fills = frames.records.filter(|r| matches!(AnyMessage::decode(r.frame, 0), Ok(AnyMessage::ExecutionReport(report)) if report.status() == OrderStatus::Filled && report.fill_qty_value().mantissa() > 0)).count();
        assert!(fills > 0, "a strategy backtest must see venue fills");
        Ok(())
    }
}

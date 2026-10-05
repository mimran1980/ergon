//! The region's dummy exchange on the live runtime.

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use engine::exchange::Exchange;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let streams = lab::Streams::load(lab::streams_path())?;
    let mut rt = lab::runtime(schema::TRADING_SCHEMA, &streams)?;
    let exchange = Exchange::new(rt.ctx())?;
    rt.run(exchange)?;
    Ok(())
}

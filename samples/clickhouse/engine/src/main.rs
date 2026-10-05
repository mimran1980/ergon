//! One region's trading engine: [`engine::agent::Engine`] on the live runtime,
//! with the lab's registry as its directory and followed for new feeds.

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use engine::agent::Engine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (streams, watch) = lab::Watch::start(lab::streams_path())?;
    let mut rt = lab::runtime(schema::TRADING_SCHEMA, &streams)?;
    let engine = Engine::new(rt.ctx(), streams)?.watching(watch);
    rt.run(engine)?;
    Ok(())
}

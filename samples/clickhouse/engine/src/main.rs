//! One region's trading engine: [`engine::agent::Engine`] on the live runtime.

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use engine::agent::Engine;
use ergon_runtime::rt::Runtime;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut rt = Runtime::from_env(schema::TRADING_SCHEMA)?;
    let engine = Engine::new(rt.ctx())?;
    rt.run(engine)?;
    Ok(())
}

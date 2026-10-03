//! The region's dummy exchange on the live runtime.

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use engine::exchange::Exchange;
use ergon_runtime::rt::Runtime;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut rt = Runtime::from_env(schema::TRADING_SCHEMA)?;
    let exchange = Exchange::new(rt.ctx())?;
    rt.run(exchange)?;
    Ok(())
}

//! The node's shared Aeron media driver: Aeron's C driver (no JIT or garbage
//! collector on the network threads), shared by every application on the
//! node and by the node's archive.
//!
//! Configured entirely by Aeron's own environment variables, read when the
//! context is made, for example:
//!
//! * `AERON_DIR`: the driver's directory, on a volume every pod on the node mounts.
//! * `AERON_THREADING_MODE`: `DEDICATED` (the default) runs the conductor,
//!   sender and receiver on three threads.
//! * `AERON_CONDUCTOR_IDLE_STRATEGY`, `AERON_SENDER_IDLE_STRATEGY`,
//!   `AERON_RECEIVER_IDLE_STRATEGY`: `spin` (busy spin), `noop`, `yield`,
//!   `sleep-ns` or `backoff`.
//!
//! The conductor runs on this thread; the sender and receiver on their own.

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use rusteron_media_driver::{AeronDriver, AeronDriverContext};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let context = AeronDriverContext::new()?;
    context.print_configuration();
    let driver = AeronDriver::new(&context)?;
    driver.start(true)?;
    loop {
        driver.main_idle_strategy(driver.main_do_work()?);
    }
}

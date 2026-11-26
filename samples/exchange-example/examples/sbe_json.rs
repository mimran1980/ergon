//! `sbe_json`: render Binance spot SBE responses as the JSON the REST API
//! returns for the same request.
//!
//! Each argument is a file holding one SBE response body (as fetched with
//! `Accept: application/sbe` and `X-MBX-SBE: 3:5`); one JSON document is printed
//! per file. Timestamps are microseconds, as the SBE API sends them.
//!
//! Run: `cargo run --example sbe_json -- tests/fixtures/spot-json/depth.sbe`

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut paths = std::env::args().skip(1).peekable();
    if paths.peek().is_none() {
        return Err("usage: sbe_json <response.sbe>...".into());
    }
    for path in paths {
        let frame = std::fs::read(&path)?;
        println!("{}", exchange_example::sbe_json::render(&frame)?);
    }
    Ok(())
}

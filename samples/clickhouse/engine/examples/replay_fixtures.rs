//! Regenerate the engine's replay fixtures, or with `--check` fail when the
//! checked-in ones no longer match what the generator and the engine make.

use std::path::Path;
use std::process::ExitCode;

use engine::replay::{self, Market};

fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let check = std::env::args().any(|a| a == "--check");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let inbound = replay::inbound(&Market::FIXTURE);
    let outbound = replay::run(inbound.clone())?;
    let files = [
        ("engine-inbound.bin", inbound),
        ("engine-outbound.golden", outbound),
    ];
    if !check {
        for (name, bytes) in &files {
            std::fs::write(dir.join(name), bytes)?;
            println!("wrote {name}: {} bytes", bytes.len());
        }
        return Ok(ExitCode::SUCCESS);
    }
    let mut ok = true;
    for (name, bytes) in &files {
        if std::fs::read(dir.join(name)).ok().as_ref() != Some(bytes) {
            eprintln!("{name} is stale: run `just engine-fixtures`");
            ok = false;
        }
    }
    Ok(if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

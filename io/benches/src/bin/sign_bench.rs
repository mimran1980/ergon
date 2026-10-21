//! Cost of signing one REST request: `params`, `&timestamp=…` and
//! `&signature=…` appended to a reused buffer, as `HttpConn::signed_request`
//! does inside the TLS record. Best of `--reps`, `--iters` requests each.
//!
//! ```text
//! sign_bench --iters 200000 --reps 5
//! ```

use std::hint::black_box;
use std::process::ExitCode;
use std::time::Instant;

use ergon_io::auth::{self, Signer};
use io_bench::args::Args;
use ring::signature::Ed25519KeyPair;

/// A Binance new-order parameter string (without timestamp and signature).
const PARAMS: &str = "symbol=BTCUSDT&side=BUY&type=LIMIT&timeInForce=GTC&quantity=0.00100000\
&price=50000.00&newClientOrderId=x-ergon-0000000000000001&recvWindow=5000";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("sign-bench: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let a = Args::from_env();
    let iters: u32 = a.num_or("iters", 200_000)?;
    let reps: u32 = a.num_or("reps", 5)?;
    if let Some(c) = a.list("cpu", "").first() {
        io_bench::clock::pin_current_thread(c.parse()?)?;
    }
    let rng = ring::rand::SystemRandom::new();
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).map_err(|e| e.to_string())?;
    let signers = [
        (
            "hmac-sha256 (ring)",
            Signer::hmac_sha256(
                b"NhqPtmdSJYdKjVHjA7PZj4Mge3R5YNiP1e3UZjInClVN65XAbvqqM6A7H5fATj0j",
            ),
        ),
        ("ed25519 (ring)", Signer::ed25519_pkcs8(pkcs8.as_ref())?),
    ];
    println!("| signer | ns per signed request (best of {reps}) | output bytes |\n|---|---|---|");
    for (name, signer) in &signers {
        let mut out = Vec::with_capacity(1024);
        let mut best = f64::MAX;
        for _ in 0..reps {
            let t0 = Instant::now();
            for i in 0..iters {
                out.clear();
                auth::append_signed_params(
                    &mut out,
                    PARAMS,
                    1_760_400_000_000 + u64::from(i),
                    signer,
                );
                black_box(&out);
            }
            best = best.min(t0.elapsed().as_nanos() as f64 / f64::from(iters));
        }
        println!("| {name} | {best:.0} | {} |", out.len());
    }
    println!("\nparams: {} bytes", PARAMS.len());
    Ok(())
}

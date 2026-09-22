//! Shared parser/validation/status CLI for `recording/v1` config.
//!
//! The Python config watcher uses this binary to validate the exact bytes
//! it is about to apply, keeping one parser across every consumer.

use std::io::Read;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let mut mode = String::from("validate");
    let mut path = String::new();
    let mut permanent: Vec<String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" => {
                i += 1;
                mode = args.get(i).cloned().unwrap_or_default();
            }
            "--file" => {
                i += 1;
                path = args.get(i).cloned().unwrap_or_default();
            }
            "--permanent" => {
                i += 1;
                permanent = args
                    .get(i)
                    .cloned()
                    .unwrap_or_default()
                    .split(',')
                    .map(str::to_string)
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            "--help" | "-h" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("unknown argument: {other}");
                return ExitCode::from(2);
            }
        }
        i += 1;
    }

    if path.is_empty() {
        eprintln!("--file <path> is required");
        return ExitCode::from(2);
    }

    let mut bytes = Vec::new();
    if path == "-" {
        if std::io::stdin().read_to_end(&mut bytes).is_err() {
            eprintln!("failed to read stdin");
            return ExitCode::from(2);
        }
    } else {
        match std::fs::read(&path) {
            Ok(data) => bytes = data,
            Err(e) => {
                eprintln!("failed to read {path}: {e}");
                return ExitCode::from(2);
            }
        }
    }

    let perm_refs: Vec<&str> = permanent.iter().map(String::as_str).collect();
    match ergo_clickhouse_persist::config::validate(&bytes, &perm_refs) {
        Ok(cfg) => {
            if mode == "status" {
                println!("api_version: {}", cfg.api_version);
                println!("digest: {:016x}", cfg.digest);
                println!("rules: {}", cfg.rules.len());
                for r in &cfg.rules {
                    println!(
                        "  {}/{}/{} enabled={} row_ttl={:?} idle_table_ttl={:?}",
                        r.process, r.instance, r.table, r.enabled, r.row_ttl, r.idle_table_ttl
                    );
                }
            } else {
                println!(
                    "OK recording/v1 digest={:016x} rules={}",
                    cfg.digest,
                    cfg.rules.len()
                );
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("INVALID: {e}");
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!(
        "recording-config — recording/v1 validation CLI\n\n\
         USAGE:\n  \
         recording-config --file <path|-> [--mode validate|status] [--permanent t1,t2]\n\n\
         Validates the exact bytes supplied; exits nonzero on invalid config.\n\
         `status` prints the parsed digest and effective rules."
    );
}

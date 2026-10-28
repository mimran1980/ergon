//! Prints the media driver's counters (`AERON_DIR`) whose label contains the
//! filter (default `rusteron-dpdk`: the DPDK transport's TX/RX, ARP and reject
//! counters), one `<value> <type> <label>` line each. A diagnostic for the
//! benchmark lab: where a DPDK frame went missing.
//!
//! counters [filter]
//!
//! `counters loss` prints instead one `key=value;...` line of every counter
//! that records loss or its repair: Aeron's NAKs, retransmits, short sends,
//! loss gap fills, invalid packets and errors, and the DPDK transport's drop
//! counters summed over its ports. lab.sh takes it before and after each run.
//! `counters errors` prints the driver's distinct error log and loss report.
//!
//! Every mode reads the driver's `CnC` file directly, as no client of the
//! driver: so it reads a driver that has stopped too (until the next one
//! deletes the directory), such as the one a lab client embedded
//! (`HARNESS_DRIVER=invoker`), which leaves every counter in place at exit.

// Only `aeron_dir` of the shared helpers is used here.
#[allow(dead_code)]
#[path = "../lab.rs"]
mod lab;

use lab::aeron_dir;
use rusteron_client::AeronCnc;
use std::error::Error;

/// Aeron's system counters that count loss or its repair, by label prefix.
const AERON_LOSS: [(&str, &str); 8] = [
    ("NAKs sent", "naks-sent"),
    ("NAKs received", "naks-received"),
    ("Retransmits sent", "retransmits-sent"),
    ("Short sends", "short-sends"),
    ("Loss gap fills", "loss-gap-fills"),
    ("Invalid packets", "invalid-packets"),
    ("Errors", "errors"),
    ("Failed offers to ReceiverProxy", "failed-receiver-offers"),
];
/// The DPDK transport's drop counters, by the name ending their label
/// (`missed-packets` mirrors the same NIC miss as `nic-rx-missed`, so only
/// the latter), and `rx-burst-full` and `tx-paced`, saturation signals rather
/// than drops, reported under a `sat-` key. `queue-drop` counts frames to one
/// of our addresses at a port nobody listens on (answered with ICMP port
/// unreachable, as the kernel counts UDP NoPorts): after an endpoint closes,
/// its peer's frames in flight land there, so it is reported under `noport-`.
/// `arp-miss` counts frames refused while a next hop's address is resolved
/// (Aeron retries them, as short sends) or dropped once it failed.
/// `tx-queued` counts frames a full TX ring refused and the transport queued
/// to send first: saturation, not loss. `rx-ring-full-suspected` counts polls
/// that drained within a burst of a full receive ring: a ring that full may
/// have overflowed unseen (MANA), so it is reported beside the loss counters.
const DPDK_LOSS: [&str; 14] = [
    "nobufs",
    "tx-eagain",
    "arp-miss",
    "queue-drop",
    "checksum-failure",
    "multi-segment",
    "nic-rx-errors",
    "nic-rx-missed",
    "nic-rx-nombuf",
    "peer-mtu-too-large",
    "rx-burst-full",
    "tx-paced",
    "tx-queued",
    "rx-ring-full-suspected",
];

fn main() -> Result<(), Box<dyn Error>> {
    let filter = std::env::args().nth(1).unwrap_or_else(|| "rusteron-dpdk".into());
    if filter == "loss" {
        return loss();
    }
    if filter == "errors" {
        return errors();
    }
    AeronCnc::open(&aeron_dir()?)?
        .counters_reader()
        .foreach_counter_fn(|value, _id, type_id, _key, label| {
            if label.contains(filter.as_str()) {
                println!("{value:>12} {type_id:>4} {label}");
            }
        });
    Ok(())
}

/// The driver's distinct error log and loss report, read from its `CnC` file
/// (also after the driver stopped, until the next one deletes the directory):
/// what Aeron's `errors` counter and its NAKs were about.
fn errors() -> Result<(), Box<dyn Error>> {
    let cnc = AeronCnc::open(&aeron_dir()?)?;
    cnc.error_log_read_fn(
        |count: i32, first_ns: i64, last_ns: i64, error: &str| {
            println!(
                "error count={count} first={first_ns} last={last_ns}: {}",
                error.replace('\n', " | ")
            );
        },
        0,
    );
    cnc.loss_reporter_read_fn(
        |count: i64, bytes: i64, first_ns: i64, last_ns: i64, session: i32, stream: i32, channel: &str, source: &str| {
            println!(
                "loss count={count} bytes={bytes} first={first_ns} last={last_ns} session={session} stream={stream} channel={channel} source={source}"
            );
        },
    )?;
    Ok(())
}

fn loss() -> Result<(), Box<dyn Error>> {
    let cnc = AeronCnc::open(&aeron_dir()?)?;
    let mut totals: Vec<(String, i64)> = AERON_LOSS
        .iter()
        .map(|(_, k)| ((*k).to_string(), 0))
        .chain(DPDK_LOSS.iter().map(|k| (dpdk_key(k), 0)))
        .collect();
    cnc.counters_reader()
        .foreach_counter_fn(|value, _id, _type_id, _key, label| {
            let key = if label.starts_with("rusteron-dpdk") {
                label
                    .split_whitespace()
                    .find(|word| DPDK_LOSS.contains(word))
                    .map(dpdk_key)
            } else {
                AERON_LOSS
                    .iter()
                    .find(|(prefix, _)| label.starts_with(prefix))
                    .map(|(_, k)| (*k).to_string())
            };
            if let Some(key) = key
                && let Some(total) = totals.iter_mut().find(|(k, _)| *k == key)
            {
                total.1 += value;
            }
        });
    let line: Vec<String> = totals.iter().map(|(k, v)| format!("{k}={v}")).collect();
    println!("{}", line.join(";"));
    Ok(())
}

/// `dpdk-<name>` for a drop, `sat-dpdk-<name>` for a saturation signal,
/// `noport-dpdk-<name>` for frames to a closed port.
fn dpdk_key(name: &str) -> String {
    if name == "rx-burst-full" || name == "tx-paced" || name == "tx-queued" || name == "rx-ring-full-suspected" {
        format!("sat-dpdk-{name}")
    } else if name == "queue-drop" {
        format!("noport-dpdk-{name}")
    } else {
        format!("dpdk-{name}")
    }
}

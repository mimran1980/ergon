//! Native prepared-recording latency vs a direct in-memory publication.
//!
//! This is a sample-local harness, not the repository-root SBE gate.
//! Laptop numbers must not be labelled as HFT production latency.

use ergo_clickhouse_persist::persist::{EncodeError, Persistable, RecordOutcome, RowWriter};
use ergo_clickhouse_persist::protocol::Policy;
use ergo_clickhouse_persist::registration::{RecorderConfig, RecorderSession, TransportConfig};
use ergo_clickhouse_persist::schema::{RowSchema, TypeCode, ValueSchema};
use std::hint::black_box;
use std::time::Instant;

static SCHEMA: RowSchema = RowSchema {
    columns: &[
        ValueSchema::scalar(TypeCode::U32),
        ValueSchema::scalar(TypeCode::I64),
        ValueSchema::scalar(TypeCode::U64),
    ],
};

struct Tick {
    instrument: u32,
    price: i64,
    update_id: u64,
}

impl Persistable for Tick {
    fn schema() -> &'static RowSchema {
        &SCHEMA
    }
    fn encoded_len(&self) -> Result<usize, EncodeError> {
        Ok(20)
    }
    fn encode(&self, out: &mut RowWriter<'_>) -> Result<(), EncodeError> {
        out.write_u32(self.instrument)?;
        out.write_i64(self.price)?;
        out.write_u64(self.update_id)?;
        Ok(())
    }
}

fn session() -> RecorderSession {
    RecorderSession::connect(RecorderConfig {
        process: "bench".into(),
        instance: "0".into(),
        build: "bench".into(),
        max_record_bytes: 4096,
        diagnostics_quota_bytes_per_sec: 0,
        permanent: TransportConfig::Memory {
            slots: 4096,
            slot_bytes: 256,
        },
        diagnostics: TransportConfig::Memory {
            slots: 64,
            slot_bytes: 256,
        },
    })
    .expect("connect")
}

fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn main() {
    let mut s = session();
    let table = s.table::<Tick>("ticks", Policy::Permanent).expect("table");
    table.slot().set_enabled(table.policy_id(), 1);
    let mut w = s.writer().expect("writer");
    let tick = Tick {
        instrument: 7,
        price: 114_562_300_000_000,
        update_id: 1,
    };
    for _ in 0..10_000 {
        let _ = w.record(&table, &tick, 1);
    }

    const N: usize = 100_000;
    let mut samples = Vec::with_capacity(N);
    let start = Instant::now();
    for i in 0..N {
        let t0 = Instant::now();
        let outcome = w.record(
            &table,
            black_box(&Tick {
                instrument: 7,
                price: 114_562_300_000_000,
                update_id: i as u64,
            }),
            1,
        );
        samples.push(t0.elapsed().as_nanos() as u64);
        assert!(matches!(
            outcome,
            RecordOutcome::Published | RecordOutcome::Dropped(_)
        ));
    }
    let elapsed = start.elapsed();
    samples.sort_unstable();
    println!(
        "enabled n={N} p50={}ns p99={}ns p99.9={}ns throughput={:.0}/s published={}",
        percentile(&samples, 0.50),
        percentile(&samples, 0.99),
        percentile(&samples, 0.999),
        N as f64 / elapsed.as_secs_f64(),
        w.counters().published,
    );

    table.slot().set_disabled();
    let mut disabled_evals = 0u64;
    let d0 = Instant::now();
    for i in 0..N {
        let outcome = w.record_with(&table, 1, |_row| {
            disabled_evals += 1;
            let t = Tick {
                instrument: 7,
                price: i as i64,
                update_id: i as u64,
            };
            t.encode(_row)
        });
        assert_eq!(outcome, RecordOutcome::Disabled);
    }
    println!(
        "disabled n={N} elapsed={}ns payload_evals={disabled_evals} (must be 0)",
        d0.elapsed().as_nanos(),
    );
    assert_eq!(disabled_evals, 0, "disabled path evaluated a payload");
}

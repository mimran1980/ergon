//! Record encoding, capture parsing and percentile maths.

use io_bench::record::{Record, parse_capture, read_records, write_records};
use io_bench::stats::{percentile, summarize};

#[test]
fn record_round_trip() {
    let r = Record {
        variant: 2,
        conn: 1,
        stream: 4,
        pos: 3,
        hw: 1,
        id: -7,
        src_ns: 1,
        src2_ns: 2,
        rx_sw_ns: 3,
        rx_hw_ns: 4,
        user_ns: 5,
        read_seq: u64::MAX,
    };
    assert_eq!(Record::decode(&r.encode()), r);
    let dir = std::env::temp_dir().join(format!("io-bench-rec-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("r.bin");
    write_records(&p, &[r, Record::default()]).unwrap();
    assert_eq!(read_records(&p).unwrap(), vec![r, Record::default()]);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_record_file_cut_mid_record_is_an_error_not_a_shorter_run() {
    let r = Record {
        id: 7,
        ..Record::default()
    };
    let dir = std::env::temp_dir().join(format!("io-bench-trunc-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("r.bin");

    // One whole record and one byte of the next: a corrupt file. It used to
    // load as one complete record, so a truncated capture was summarised as a
    // finished run.
    let mut bytes = r.encode().to_vec();
    bytes.push(0);
    std::fs::write(&p, &bytes).unwrap();
    let err = read_records(&p).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
    assert!(err.to_string().contains("truncated record"), "{err}");

    // An empty file is a clean end of file with no records, not an error.
    std::fs::write(&p, []).unwrap();
    assert_eq!(read_records(&p).unwrap(), vec![]);

    // Whole records only.
    write_records(&p, &[r]).unwrap();
    assert_eq!(read_records(&p).unwrap(), vec![r]);

    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn capture_parse_ignores_truncated_tail() {
    let mut b = Vec::new();
    for (t, p) in [(10i64, &b"abc"[..]), (20, b"")] {
        b.extend_from_slice(&t.to_le_bytes());
        b.extend_from_slice(&(p.len() as u32).to_le_bytes());
        b.extend_from_slice(p);
    }
    b.extend_from_slice(&30i64.to_le_bytes());
    b.extend_from_slice(&5u32.to_le_bytes());
    b.extend_from_slice(b"ab");
    let v = parse_capture(&b);
    assert_eq!(v, vec![(10, &b"abc"[..]), (20, &b""[..])]);
}

#[test]
fn nearest_rank_percentiles() {
    let mut v: Vec<i64> = (1..=1000).rev().collect();
    let s = summarize(&mut v).unwrap();
    assert_eq!(
        (s.p50, s.p90, s.p99, s.p999, s.min, s.max),
        (500, 900, 990, 999, 1, 1000)
    );
    assert!((s.mean - 500.5).abs() < 1e-9);
    assert_eq!(percentile(&[], 0.5), 0);
    assert_eq!(percentile(&[42], 0.999), 42);
    assert!(summarize(&mut []).is_none());
}

#[test]
fn records_and_text_are_read_from_their_zstd_copies() -> Result<(), Box<dyn std::error::Error>> {
    use io_bench::record::{exists_maybe_compressed, read_records, read_text, write_records};
    let dir = std::env::temp_dir().join(format!("zst-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    let records: Vec<_> = (1..=1000)
        .map(|i| io_bench::record::Record {
            id: i,
            src_ns: i * 100,
            user_ns: i * 100 + 45_000,
            ..io_bench::record::Record::default()
        })
        .collect();
    let plain = dir.join("records.bin");
    write_records(&plain, &records)?;
    let z = zstd::encode_all(std::fs::File::open(&plain)?, 10)?;
    std::fs::write(dir.join("records.bin.zst"), z)?;
    std::fs::remove_file(&plain)?;
    assert_eq!(read_records(&plain)?, records);
    std::fs::write(
        dir.join("trace.txt.zst"),
        zstd::encode_all("a trace line\n".as_bytes(), 10)?,
    )?;
    assert!(exists_maybe_compressed(&dir.join("trace.txt")));
    assert_eq!(read_text(&dir.join("trace.txt"))?, "a trace line\n");
    assert!(read_records(&dir.join("missing.bin")).is_err());
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}

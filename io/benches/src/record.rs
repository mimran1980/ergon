//! Fixed-size per-message records, kept in a preallocated vector during a run and
//! written to disk afterwards.

use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

/// One received message (or one ping-pong round trip).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Record {
    /// Index into the run's variant list.
    pub variant: u8,
    /// Connection index within the variant.
    pub conn: u8,
    /// Stream index into the run's stream list.
    pub stream: u16,
    /// Position of this message among those decoded from the same read.
    pub pos: u16,
    /// 1 if `rx_hw_ns` is a NIC hardware timestamp.
    pub hw: u8,
    /// Unique id within the stream (trade id, update id or lab sequence).
    pub id: i64,
    /// Source timestamp in ns (Binance `E`, or the lab sender's send time).
    pub src_ns: i64,
    /// Secondary source timestamp in ns (Binance `T`, the capture time, or
    /// when a paced ping-pong or HTTP request was due; 0 if none).
    pub src2_ns: i64,
    /// Kernel software receive timestamp (`CLOCK_REALTIME`, ns).
    pub rx_sw_ns: i64,
    /// NIC hardware receive timestamp (raw PHC, ns), 0 if none.
    pub rx_hw_ns: i64,
    /// User-side timestamp after decoding (`CLOCK_REALTIME`, ns).
    pub user_ns: i64,
    /// Sequence number of the read that delivered this message.
    pub read_seq: u64,
}

/// Encoded size of one record.
pub const RECORD_LEN: usize = 64;

impl Record {
    /// Encodes to little-endian bytes.
    #[must_use]
    pub fn encode(&self) -> [u8; RECORD_LEN] {
        let mut b = [0u8; RECORD_LEN];
        b[0] = self.variant;
        b[1] = self.conn;
        b[2..4].copy_from_slice(&self.stream.to_le_bytes());
        b[4..6].copy_from_slice(&self.pos.to_le_bytes());
        b[6] = self.hw;
        b[8..16].copy_from_slice(&self.id.to_le_bytes());
        b[16..24].copy_from_slice(&self.src_ns.to_le_bytes());
        b[24..32].copy_from_slice(&self.src2_ns.to_le_bytes());
        b[32..40].copy_from_slice(&self.rx_sw_ns.to_le_bytes());
        b[40..48].copy_from_slice(&self.rx_hw_ns.to_le_bytes());
        b[48..56].copy_from_slice(&self.user_ns.to_le_bytes());
        b[56..64].copy_from_slice(&self.read_seq.to_le_bytes());
        b
    }

    /// Decodes from little-endian bytes.
    #[must_use]
    pub fn decode(b: &[u8; RECORD_LEN]) -> Self {
        let i = |r: std::ops::Range<usize>| {
            let mut a = [0u8; 8];
            a.copy_from_slice(&b[r]);
            i64::from_le_bytes(a)
        };
        Self {
            variant: b[0],
            conn: b[1],
            stream: u16::from_le_bytes([b[2], b[3]]),
            pos: u16::from_le_bytes([b[4], b[5]]),
            hw: b[6],
            id: i(8..16),
            src_ns: i(16..24),
            src2_ns: i(24..32),
            rx_sw_ns: i(32..40),
            rx_hw_ns: i(40..48),
            user_ns: i(48..56),
            read_seq: i(56..64) as u64,
        }
    }
}

/// Writes records to `path`.
///
/// # Errors
/// File I/O failure.
pub fn write_records(path: &Path, records: &[Record]) -> std::io::Result<()> {
    let mut w = BufWriter::new(std::fs::File::create(path)?);
    for r in records {
        w.write_all(&r.encode())?;
    }
    w.flush()
}

/// Opens `path`, or, when only that exists, its zstd-compressed copy
/// `path.zst` (results are compressed on the VM before they are copied).
///
/// # Errors
/// Neither file can be opened, or the compressed one is not zstd.
pub fn open_maybe_compressed(path: &Path) -> std::io::Result<Box<dyn Read>> {
    match std::fs::File::open(path) {
        Ok(f) => Ok(Box::new(f)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let f = std::fs::File::open(compressed(path)).map_err(|_| e)?;
            Ok(Box::new(zstd::stream::read::Decoder::new(f)?))
        }
        Err(e) => Err(e),
    }
}

/// `path` with `.zst` appended.
fn compressed(path: &Path) -> std::path::PathBuf {
    let mut z = path.as_os_str().to_owned();
    z.push(".zst");
    z.into()
}

/// Reads a text file, or its `.zst` copy ([`open_maybe_compressed`]).
///
/// # Errors
/// As [`open_maybe_compressed`], or the text is not UTF-8.
pub fn read_text(path: &Path) -> std::io::Result<String> {
    let mut s = String::new();
    open_maybe_compressed(path)?.read_to_string(&mut s)?;
    Ok(s)
}

/// Whether `path` or its `.zst` copy exists.
#[must_use]
pub fn exists_maybe_compressed(path: &Path) -> bool {
    path.exists() || compressed(path).exists()
}

/// Reads all records from `path` (or `path.zst`).
///
/// # Errors
/// File I/O failure or a truncated file. A clean end of file is only an empty
/// read; a file that stops part-way through a record is corrupt and must not
/// be summarised as a complete run.
pub fn read_records(path: &Path) -> std::io::Result<Vec<Record>> {
    let mut r = BufReader::new(open_maybe_compressed(path)?);
    let mut out = Vec::new();
    let mut b = [0u8; RECORD_LEN];
    loop {
        // Read one byte first: read_exact alone cannot tell "no more records"
        // from "the file was cut inside a record".
        if r.read(&mut b[..1])? == 0 {
            return Ok(out);
        }
        r.read_exact(&mut b[1..]).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("truncated record after {} whole records: {e}", out.len()),
            )
        })?;
        out.push(Record::decode(&b));
    }
}

/// Reads a little-endian `i64` at `at`.
#[must_use]
pub fn read_i64(b: &[u8], at: usize) -> i64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[at..at + 8]);
    i64::from_le_bytes(a)
}

/// Parses a capture file (`capture_ns i64 | len u32 | payload` entries) into
/// `(capture_ns, payload)` pairs. A truncated final entry is ignored.
#[must_use]
pub fn parse_capture(bytes: &[u8]) -> Vec<(i64, &[u8])> {
    let mut v = Vec::new();
    let mut i = 0;
    while i + 12 <= bytes.len() {
        let t = read_i64(bytes, i);
        let len =
            u32::from_le_bytes([bytes[i + 8], bytes[i + 9], bytes[i + 10], bytes[i + 11]]) as usize;
        if i + 12 + len > bytes.len() {
            break;
        }
        v.push((t, &bytes[i + 12..i + 12 + len]));
        i += 12 + len;
    }
    v
}

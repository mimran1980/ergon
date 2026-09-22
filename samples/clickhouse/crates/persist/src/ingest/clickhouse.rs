//! Minimal ClickHouse RowBinary-over-HTTP client, storage mapping, schema
//! evolution, and batched inserts.
//!
//! Only what the ingester needs: DDL, `system.columns` reconciliation,
//! explicit-column RowBinary inserts with synchronous acknowledgement, and
//! TTL/storage-policy management. No async runtime; one connection at a
//! time from the single ingester owner.

use super::BatchBuffer;
use crate::schema::{RowSchema, TypeCode, ValueSchema};

/// Storage/DDL failure.
#[derive(Clone, Debug)]
pub enum ChError {
    // (variants below)
    /// HTTP/transport failure (ambiguous: retry with the same event IDs).
    Transport(String),
    /// Server refused the statement.
    Server { code: u16, message: String },
    /// Column exists with an incompatible type; the field is omitted.
    IncompatibleColumn {
        column: String,
        existing: String,
        wanted: String,
    },
}

impl std::fmt::Display for ChError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(m) => write!(f, "transport: {m}"),
            Self::Server { code, message } => write!(f, "server {code}: {message}"),
            Self::IncompatibleColumn {
                column,
                existing,
                wanted,
            } => {
                write!(
                    f,
                    "incompatible column {column}: existing {existing}, wanted {wanted}"
                )
            }
        }
    }
}

impl std::error::Error for ChError {}

/// Minimal HTTP RowBinary client (single-threaded).
pub struct ClickHouse {
    base_url: String,
    database: String,
    user: String,
    password: String,
    agent: ureq::Agent,
}

impl ClickHouse {
    /// Connect to `http://host:port` with a database and optional auth.
    ///
    /// Auth is sent per-request via X-ClickHouse headers, matching the
    /// native HTTP interface without a per-agent credential binding.
    #[must_use]
    pub fn new(base_url: &str, database: &str, user: &str, password: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            database: database.to_string(),
            user: user.to_string(),
            password: password.to_string(),
            agent: ureq::AgentBuilder::new()
                .timeout_connect(std::time::Duration::from_secs(5))
                .timeout(std::time::Duration::from_secs(120))
                .build(),
        }
    }

    /// Per-request headers (auth + database).
    fn headers(&self, req: ureq::Request) -> ureq::Request {
        let req = req.set("X-ClickHouse-User", &self.user);
        if !self.password.is_empty() {
            req.set("X-ClickHouse-Key", &self.password)
        } else {
            req
        }
    }

    /// Execute a DDL/statement or query; the statement travels in the
    /// request body (native HTTP interface, no URL encoding limits).
    pub fn exec(&self, query: &str) -> Result<String, ChError> {
        match self
            .headers(
                self.agent
                    .post(&format!("{}/?database={}", self.base_url, self.database)),
            )
            .send_string(query)
        {
            Ok(resp) => resp
                .into_string()
                .map_err(|e| ChError::Transport(e.to_string())),
            Err(ureq::Error::Status(code, resp)) => {
                let msg = resp.into_string().unwrap_or_default();
                Err(ChError::Server { code, message: msg })
            }
            Err(e) => Err(ChError::Transport(e.to_string())),
        }
    }

    /// Insert a batch with explicit columns; body is RowBinary.
    pub fn insert(
        &self,
        table: &str,
        columns: &[String],
        batch: &BatchBuffer,
    ) -> Result<(), ChError> {
        self.insert_raw(table, columns, batch.body())
    }

    /// Insert pre-encoded RowBinary bytes under explicit columns. The
    /// INSERT statement travels as the `query` parameter and the binary
    /// body as the request body.
    pub fn insert_raw(&self, table: &str, columns: &[String], body: &[u8]) -> Result<(), ChError> {
        let col_list = columns
            .iter()
            .map(|c| format!("`{c}`"))
            .collect::<Vec<_>>()
            .join(", ");
        let query = format!("INSERT INTO {} ({}) FORMAT RowBinary", table, col_list);
        let url = format!(
            "{}/?database={}&query={}",
            self.base_url,
            self.database,
            urlencode(&query)
        );
        let req = self.headers(self.agent.post(&url));
        let resp = req.send_bytes(body);
        match resp {
            Ok(_) => Ok(()),
            Err(ureq::Error::Status(code, resp)) => {
                let msg = resp.into_string().unwrap_or_default();
                Err(ChError::Server { code, message: msg })
            }
            Err(e) => Err(ChError::Transport(e.to_string())),
        }
    }
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Storage type name for a value schema.
#[must_use]
pub fn ch_type(v: &ValueSchema) -> String {
    let base = match v.ty {
        TypeCode::Bool => "UInt8".into(),
        TypeCode::I8 => "Int8".into(),
        TypeCode::I16 => "Int16".into(),
        TypeCode::I32 => "Int32".into(),
        TypeCode::I64 => "Int64".into(),
        TypeCode::U8 => "UInt8".into(),
        TypeCode::U16 => "UInt16".into(),
        TypeCode::U32 => "UInt32".into(),
        TypeCode::U64 => "UInt64".into(),
        TypeCode::F32 => "Float32".into(),
        TypeCode::F64 => "Float64".into(),
        TypeCode::Decimal => {
            if v.precision <= 9 {
                format!("Decimal32({})", v.scale)
            } else if v.precision <= 18 {
                format!("Decimal64({})", v.scale)
            } else {
                format!("Decimal128({})", v.scale)
            }
        }
        TypeCode::Utf8 | TypeCode::Bytes => "String".into(),
        TypeCode::TimestampNs => "DateTime64(9, 'UTC')".into(),
    };
    let arr = if v.is_array {
        format!("Array({base})")
    } else {
        base
    };
    // Nullable scalars only: Nullable(Array) is deliberately avoided.
    if v.nullable && !v.is_array {
        format!("Nullable({arr})")
    } else {
        arr
    }
}

/// Binary encoders for one value in ClickHouse RowBinary.
pub mod bin {
    /// LEB128 unsigned (ClickHouse String length prefix).
    pub fn varuint(out: &mut Vec<u8>, mut v: u64) {
        loop {
            let b = (v & 0x7F) as u8;
            v >>= 7;
            if v == 0 {
                out.push(b);
                break;
            }
            out.push(b | 0x80);
        }
    }

    /// String value (length-prefixed).
    pub fn string(out: &mut Vec<u8>, v: &[u8]) {
        varuint(out, v.len() as u64);
        out.extend_from_slice(v);
    }

    /// Nullable flag byte: 1 = NULL, 0 = value follows (ClickHouse
    /// RowBinary semantics).
    pub fn nullable_prefix(out: &mut Vec<u8>, present: bool) {
        out.push(u8::from(!present));
    }

    /// Array size prefix: varint (LEB128) element count, matching the
    /// verified server behavior (verified against ClickHouse 24.8).
    pub fn array_prefix(out: &mut Vec<u8>, count: usize) {
        varuint(out, count as u64);
    }
}

/// Build the backing-table DDL for a layout.
///
/// ReplacingMergeTree keyed on capture time + event identity; routing
/// fields may prepend the order key for instrument/time pruning. Storage
/// tiers: LZ4 hot, ZSTD(3) warm at 24h, ZSTD(15) cold (S3) at 30 days.
#[must_use]
pub fn create_table_ddl(
    table: &str,
    schema: &RowSchema,
    column_names: &[String],
    temporary: bool,
    order_prefix: &[&str],
) -> String {
    let mut cols = String::new();
    for (i, (name, v)) in column_names.iter().zip(schema.columns).enumerate() {
        if i > 0 {
            cols.push_str(", ");
        }
        cols.push_str(&format!("`{}` {}", name, ch_type(v)));
    }
    let mut order = order_prefix
        .iter()
        .map(|s| format!("`{s}`"))
        .collect::<Vec<_>>();
    order.push("`_record_captured_at_ns`".into());
    order.push("`_record_run_id`".into());
    order.push("`_record_writer_id`".into());
    order.push("`_record_sequence`".into());
    order.push("`_record_row_index`".into());
    // A temporary table's expiry is the per-row `_record_row_ttl_ns` column
    // (immutable per row, computed from the row's policy), not a table-level
    // TTL parameter.
    let temp_cols = ", `_record_row_ttl_ns` UInt64";
    // `_record_row_ttl_ns` is nanoseconds on the wire (the protocol field is
    // `row_ttl_ns`, and the config's `24h` default reaches it as
    // 86_400_000_000_000). Feeding that to `toIntervalSecond` — which the DDL
    // did — reads a 24-hour retention as 24 hours' worth of *seconds* and puts
    // the expiry in the year 2299. It has to be converted, not reinterpreted.
    //
    // It cannot be converted with `toIntervalNanosecond` either: adding a
    // nanosecond interval promotes the expression to `DateTime64(9)`, and
    // ClickHouse rejects a non-DateTime TTL expression outright
    // (`BAD_TTL_EXPRESSION`). So the TTL clause works in whole seconds and the
    // exact nanosecond comparison lives in the view predicate, which is the
    // correctness contract. Both were verified against ClickHouse 24.8.
    let ttl = if temporary {
        format!(
            " TTL toDateTime(intDiv(`_record_captured_at_ns`, 1000000000)) \
               + toIntervalSecond({}) DELETE",
            ttl_ns_ceiling_seconds_expr("_record_row_ttl_ns")
        )
    } else if order_prefix.contains(&"__storage_tiers") {
        " TTL toDateTime(intDiv(`_record_captured_at_ns`, 1000000000)) \
           + INTERVAL 24 HOUR TO VOLUME 'warm' \
         , toDateTime(intDiv(`_record_captured_at_ns`, 1000000000)) \
           + INTERVAL 30 DAY TO VOLUME 'cold'"
            .to_string()
    } else {
        String::new()
    };
    format!(
        "CREATE TABLE IF NOT EXISTS `{table}` \
         ({cols}, \
         `_record_captured_at_ns` UInt64, \
         `_record_run_id` UInt64, \
         `_record_writer_id` UInt16, \
         `_record_sequence` UInt64, \
         `_record_row_index` UInt32{temp_cols}) \
         ENGINE = ReplacingMergeTree \
         PARTITION BY toDate(fromUnixTimestamp64Nano(`_record_captured_at_ns`)) \
         ORDER BY ({}){ttl}",
        order.join(", ")
    )
}

/// Nanoseconds per second: the unit of the per-row `_record_row_ttl_ns`.
pub const NS_PER_SECOND: u64 = 1_000_000_000;

/// SQL expression converting a nanosecond retention column to whole seconds,
/// rounding **up**.
///
/// The backing table's `TTL` clause has to resolve to a `DateTime`, so it
/// cannot carry sub-second precision — and rounding *down* would let the
/// reclaimer delete rows that the public view still shows as live. Rounding up
/// keeps the TTL strictly more permissive than the view, which is the only
/// direction that is safe: reclamation may lag the contract, never lead it.
#[must_use]
pub fn ttl_ns_ceiling_seconds_expr(column: &str) -> String {
    format!("intDiv(`{column}` + {NS_PER_SECOND} - 1, {NS_PER_SECOND})")
}

/// Row-expiry predicate applied by a temporary table's public view.
///
/// PLAN §5: "The public view filters expired rows immediately, while a delete
/// TTL reclaims storage in the background." Both read the same per-row column,
/// so the view and the background TTL cannot disagree about what has expired —
/// the view is the correctness contract, the TTL clause is only storage
/// reclamation and may lag arbitrarily behind merges.
#[must_use]
pub fn temp_expiry_predicate() -> String {
    // Both columns are nanoseconds already, so the expiry is their plain sum.
    // Scaling the retention up to nanoseconds first (`* 1e9`) double-converts
    // it: a 2-second policy becomes ~63 years, and the view stops hiding
    // anything. Caught by the live ClickHouse test, not by inspection.
    "`_record_captured_at_ns` + `_record_row_ttl_ns` \
     > toUInt64(toUnixTimestamp64Nano(now64(9)))"
        .to_string()
}

/// Public duplicate-free view over the backing table. Names are inserted
/// as supplied (quote them if they contain reserved characters).
///
/// A `temporary` backing table additionally hides rows whose per-row expiry
/// has passed, so expiry is observable at query time rather than only after
/// the next merge.
#[must_use]
pub fn create_view_ddl(view: &str, table: &str, temporary: bool) -> String {
    if temporary {
        format!(
            "CREATE VIEW IF NOT EXISTS {view} AS SELECT * FROM {table} FINAL \
             WHERE {}",
            temp_expiry_predicate()
        )
    } else {
        format!("CREATE VIEW IF NOT EXISTS {view} AS SELECT * FROM {table} FINAL")
    }
}

/// Read `N` little-endian bytes at `pos`, bounds-checked against `payload`.
/// The slice-to-array conversion is infallible by construction once the
/// bounds check passes (the slice is exactly `N` bytes), so this is the one
/// place that assertion is made, instead of the same checked-then-`unwrap`
/// pair repeated at every fixed-width field.
fn read_le<const N: usize>(payload: &[u8], pos: usize, what: &str) -> Result<[u8; N], ChError> {
    if pos + N > payload.len() {
        return Err(ChError::Transport(format!("short {what}")));
    }
    Ok(payload[pos..pos + N]
        .try_into()
        .expect("slice length matches N by construction"))
}

/// Convert one persist OrderedRow payload into ClickHouse RowBinary,
/// appending the system identity columns.
pub fn persist_payload_to_rowbinary(
    schema: &RowSchema,
    payload: &[u8],
    meta: &crate::persist::RecordMetadata,
    run_id: u64,
    include_ttl: bool,
    ttl_ns: u64,
) -> Result<Vec<u8>, ChError> {
    let mut out = Vec::with_capacity(payload.len() + 64);
    let nullmap = schema.nullmap_len();
    if payload.len() < nullmap {
        return Err(ChError::Transport(
            "persist payload shorter than nullmap".into(),
        ));
    }
    let mut pos = nullmap;
    let mut null_bit = 0usize;
    for col in schema.columns {
        let absent = if col.nullable {
            let bit = (payload[null_bit / 8] & (1 << (null_bit % 8))) != 0;
            null_bit += 1;
            bit
        } else {
            false
        };
        if col.nullable {
            bin::nullable_prefix(&mut out, !absent);
        }
        if col.is_array {
            let count = u32::from_le_bytes(read_le(payload, pos, "array count")?) as usize;
            pos += 4;
            bin::array_prefix(&mut out, count);
            let width = match col.ty {
                TypeCode::Decimal => 16,
                t => t.fixed_width().unwrap_or(8),
            };
            let ch_width = if col.ty == TypeCode::Decimal {
                if col.precision <= 9 { 4 } else { 8 }
            } else {
                width
            };
            let nbytes = count.saturating_mul(width);
            if pos + nbytes > payload.len() {
                return Err(ChError::Transport("short array body".into()));
            }
            if col.ty == TypeCode::Decimal {
                for i in 0..count {
                    let start = pos + i * 16;
                    let mantissa =
                        i128::from_le_bytes(payload[start..start + 16].try_into().unwrap());
                    let v = i64::try_from(mantissa).unwrap_or(0);
                    out.extend_from_slice(&v.to_le_bytes());
                }
            } else {
                for i in 0..count {
                    let start = pos + i * width;
                    out.extend_from_slice(&payload[start..start + ch_width.min(width)]);
                }
            }
            pos += nbytes;
            continue;
        }
        match col.ty {
            TypeCode::Decimal => {
                let mantissa = i128::from_le_bytes(read_le(payload, pos, "decimal")?);
                pos += 16;
                if !absent {
                    if col.precision <= 9 {
                        let v = i32::try_from(mantissa).unwrap_or(0);
                        out.extend_from_slice(&v.to_le_bytes());
                    } else {
                        let v = i64::try_from(mantissa).unwrap_or(0);
                        out.extend_from_slice(&v.to_le_bytes());
                    }
                }
            }
            TypeCode::Utf8 | TypeCode::Bytes => {
                let n = u32::from_le_bytes(read_le(payload, pos, "bytes length")?) as usize;
                pos += 4;
                if pos + n > payload.len() {
                    return Err(ChError::Transport("short bytes body".into()));
                }
                if !absent {
                    bin::string(&mut out, &payload[pos..pos + n]);
                }
                pos += n;
            }
            other => {
                let w = other.fixed_width().unwrap_or(8);
                if pos + w > payload.len() {
                    return Err(ChError::Transport("short scalar".into()));
                }
                if !absent {
                    out.extend_from_slice(&payload[pos..pos + w]);
                }
                pos += w;
            }
        }
    }
    out.extend_from_slice(&meta.captured_at_ns.to_le_bytes());
    out.extend_from_slice(&run_id.to_le_bytes());
    out.extend_from_slice(&meta.writer_id.to_le_bytes());
    out.extend_from_slice(&meta.sequence.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    if include_ttl {
        out.extend_from_slice(&ttl_ns.to_le_bytes());
    }
    Ok(out)
}

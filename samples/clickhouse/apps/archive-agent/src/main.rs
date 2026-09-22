//! Registration/catalog owner + metrics for each producer pod.
//!
//! The archive-agent owns the producer pod's registration catalog: it
//! serves a local Unix-domain registration socket, persists declarations
//! in SQLite (WAL + FULL sync) on the Archive PVC, and exposes read-only
//! catalog/health/metrics endpoints on a separate loop. Registration and
//! metrics run on separate threads so a scrape cannot delay registration.
//! On the ingester pod the same binary runs in metrics-only mode beside
//! its own local driver.

use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

/// Agent configuration.
#[derive(Clone, Debug)]
pub struct AgentConfig {
    /// Unix socket path for registration requests.
    pub registration_socket: String,
    /// SQLite catalog path (on the Archive PVC).
    pub catalog_path: String,
    /// Metrics listen address (TCP, Prometheus text exposition).
    pub metrics_addr: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            registration_socket: "/tmp/ergo-recording-registration.sock".into(),
            catalog_path: "/tmp/ergo-registration-catalog.db".into(),
            metrics_addr: "127.0.0.1:9101".into(),
        }
    }
}

/// Shared agent counters (atomics; the metrics thread reads these).
#[derive(Debug, Default)]
pub struct AgentCounters {
    /// Declarations committed.
    pub declarations: AtomicU64,
    /// Acknowledged registrations.
    pub ok: AtomicU64,
    /// Rejected registrations.
    pub failed: AtomicU64,
}

/// Durable registration catalog.
pub struct RegistrationCatalog {
    conn: Connection,
    counters: Arc<AgentCounters>,
}

impl AgentCounters {
    /// Snapshot of the three counters.
    #[must_use]
    pub fn snapshot(&self) -> (u64, u64, u64) {
        (
            self.declarations.load(Ordering::Relaxed),
            self.ok.load(Ordering::Relaxed),
            self.failed.load(Ordering::Relaxed),
        )
    }
}

impl RegistrationCatalog {
    /// Open (creating schema if needed).
    pub fn open(path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS declarations (
                run_id INTEGER NOT NULL,
                seq INTEGER NOT NULL,
                kind TEXT NOT NULL,
                payload BLOB NOT NULL,
                committed_at_ns INTEGER NOT NULL,
                PRIMARY KEY (run_id, seq)
            );
            CREATE TABLE IF NOT EXISTS sessions (
                run_id INTEGER PRIMARY KEY,
                process TEXT NOT NULL,
                instance TEXT NOT NULL,
                build TEXT NOT NULL,
                started_at_ns INTEGER NOT NULL
            );",
        )?;
        Ok(Self {
            conn,
            counters: Arc::new(AgentCounters::default()),
        })
    }

    /// Shared counters handle for the metrics surface.
    #[must_use]
    pub fn counters_handle(&self) -> Arc<AgentCounters> {
        Arc::clone(&self.counters)
    }

    /// Commit one declaration durably; idempotent per (run, seq).
    pub fn commit(
        &self,
        run_id: u64,
        seq: u64,
        kind: &str,
        payload: &[u8],
    ) -> Result<(), rusqlite::Error> {
        let now = ergo_clickhouse_persist::registration::now_ns();
        self.conn.execute(
            "INSERT OR IGNORE INTO declarations VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![run_id as i64, seq as i64, kind, payload, now as i64],
        )?;
        self.counters.declarations.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Counters snapshot.
    #[must_use]
    pub fn counters(&self) -> (u64, u64, u64) {
        self.counters.snapshot()
    }
}

/// Handle one registration connection: length-prefixed SBE declarations
/// delivered as (run_id u64, seq u64, kind u8, len u32, bytes).
fn handle_connection(conn: &UnixStream, catalog: &Mutex<RegistrationCatalog>) {
    let mut conn = conn;
    let catalog = catalog.lock().expect("poisoned");
    let _ = conn.set_read_timeout(Some(std::time::Duration::from_secs(5)));
    let mut header = [0u8; 21];
    if conn.read_exact(&mut header).is_err() {
        catalog.counters.failed.fetch_add(1, Ordering::Relaxed);
        return;
    }
    let run_id = u64::from_le_bytes(header[..8].try_into().expect("run id"));
    let seq = u64::from_le_bytes(header[8..16].try_into().expect("seq"));
    let kind = header[16];
    let len = u32::from_le_bytes(header[17..21].try_into().expect("len")) as usize;
    if len > 1024 * 1024 {
        catalog.counters.failed.fetch_add(1, Ordering::Relaxed);
        let _ = conn.write_all(b"F");
        return;
    }
    let mut payload = vec![0u8; len];
    if conn.read_exact(&mut payload).is_err() {
        catalog.counters.failed.fetch_add(1, Ordering::Relaxed);
        return;
    }
    let kind_name = match kind {
        1 => "session_start",
        2 => "symbol",
        3 => "policy",
        4 => "layout",
        _ => {
            catalog.counters.failed.fetch_add(1, Ordering::Relaxed);
            let _ = conn.write_all(b"F");
            return;
        }
    };
    match catalog.commit(run_id, seq, kind_name, &payload) {
        Ok(()) => {
            catalog.counters.ok.fetch_add(1, Ordering::Relaxed);
            let _ = conn.write_all(b"A");
        }
        Err(_) => {
            catalog.counters.failed.fetch_add(1, Ordering::Relaxed);
            let _ = conn.write_all(b"F");
        }
    }
}

/// Serve registration requests on the Unix socket (blocking accept loop on
/// its own thread).
pub fn serve_registration(
    cfg: AgentConfig,
    catalog: Arc<Mutex<RegistrationCatalog>>,
) -> std::io::Result<()> {
    let _ = std::fs::remove_file(&cfg.registration_socket);
    let listener = UnixListener::bind(&cfg.registration_socket)?;
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                // Registration is control-plane only; handle inline to keep
                // ordering (single-threaded producer registration).
                handle_connection(&s, &catalog);
            }
            Err(_) => continue,
        }
    }
    Ok(())
}

/// Minimal Prometheus text exposition: health, catalog counters.
fn metrics_body(counters: &AgentCounters) -> String {
    let (decls, ok, failed) = counters.snapshot();
    format!(
        "# HELP ergo_agent_registrations_total registration declarations committed\n\
         # TYPE ergo_agent_registrations_total counter\n\
         ergo_agent_registrations_total {decls}\n\
         # HELP ergo_agent_registrations_ok_total acknowledged registrations\n\
         # TYPE ergo_agent_registrations_ok_total counter\n\
         ergo_agent_registrations_ok_total {ok}\n\
         # HELP ergo_agent_registrations_failed_total rejected registrations\n\
         # TYPE ergo_agent_registrations_failed_total counter\n\
         ergo_agent_registrations_failed_total {failed}\n\
         # HELP ergo_agent_up agent liveness\n\
         # TYPE ergo_agent_up gauge\n\
         ergo_agent_up 1\n"
    )
}

/// Serve /metrics on TCP.
pub fn serve_metrics(cfg: AgentConfig, counters: Arc<AgentCounters>) -> std::io::Result<()> {
    use std::net::TcpListener;
    let listener = TcpListener::bind(&cfg.metrics_addr)?;
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let mut buf = [0u8; 1024];
        let _ = stream.read(&mut buf);
        let body = metrics_body(&counters);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut config = AgentConfig::default();
    let mut mode = String::from("agent");
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--socket" => {
                i += 1;
                config.registration_socket = args.get(i).cloned().unwrap_or_default();
            }
            "--catalog" => {
                i += 1;
                config.catalog_path = args.get(i).cloned().unwrap_or_default();
            }
            "--metrics-addr" => {
                i += 1;
                config.metrics_addr = args.get(i).cloned().unwrap_or_default();
            }
            "--mode" => {
                i += 1;
                mode = args.get(i).cloned().unwrap_or_else(|| mode.clone());
            }
            other => {
                eprintln!("unknown argument: {other}");
                return Err("bad arguments".into());
            }
        }
        i += 1;
    }

    // Registration requests are handled inline (single ordering authority);
    // the catalog connection is mutex-guarded for that interior access.
    let catalog = Arc::new(Mutex::new(RegistrationCatalog::open(&config.catalog_path)?));
    let counters = catalog.lock().expect("poisoned").counters_handle();
    eprintln!(
        "archive-agent ({mode}): socket={} catalog={} metrics={}",
        config.registration_socket, config.catalog_path, config.metrics_addr
    );

    if mode == "metrics-only" {
        // Ingester pod: metrics beside its own local driver; no registration socket.
        serve_metrics(config, counters)?;
    } else {
        // Producer pod: registration loop owns the socket; metrics on a
        // separate thread so a scrape cannot delay registration.
        let metrics_cfg = config.clone();
        std::thread::spawn(move || {
            if let Err(e) = serve_metrics(metrics_cfg, counters) {
                eprintln!("metrics server failed: {e}");
            }
        });
        serve_registration(config, Arc::clone(&catalog))?;
    }
    Ok(())
}

//! Central ingester: replays recording streams, resolves the durable
//! catalog, projects rows, and pushes bounded batches to ClickHouse with
//! contiguous-prefix checkpoints.
//!
//! Ownership: one active ingester owns all ClickHouse DDL and writes for
//! the sample namespace. Read as much as the replay budget allows, then
//! push one RowBinary insert per flush — rows (8192), bytes (8 MiB), or
//! 100 ms, whichever comes first.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(feature = "archive")]
use std::time::Duration;
use std::time::Instant;

use ergo_clickhouse_persist::ingest::catalog::{Binding, Catalog, LifecycleRecord};
use ergo_clickhouse_persist::ingest::checkpoint::{CheckpointStore, ContiguousPrefix};
use ergo_clickhouse_persist::ingest::clickhouse::{ClickHouse, create_table_ddl, create_view_ddl};
use ergo_clickhouse_persist::ingest::lifecycle::{
    LifecycleState, OnInput, advance_drop, has_idle_policy, idle_clock_started, may_drop, on_input,
    row_expiry_ns,
};
use ergo_clickhouse_persist::ingest::{
    BatchBuffer, IngestSource, ReplayBudget, SourceError, SourceItem,
};
use ergo_clickhouse_persist::protocol::{Declaration, LayoutDeclaration, Policy, RowEncoding};

/// Ingester runtime configuration.
#[derive(Clone, Debug)]
pub struct IngesterConfig {
    /// ClickHouse base URL.
    pub clickhouse_url: String,
    /// ClickHouse database.
    pub database: String,
    /// ClickHouse user/password.
    pub user: String,
    /// ClickHouse password.
    pub password: String,
    /// Path to the durable catalog SQLite file.
    pub catalog_path: String,
    /// Path to the checkpoint SQLite file.
    pub checkpoint_path: String,
    /// Replay budget per source (bytes).
    pub replay_budget_bytes: usize,
    /// Archive segment file length used by the producer pods (pruning
    /// alignment). 0 disables pruning.
    pub archive_segment_length: usize,
    /// When pruning is enabled: prune no closer than one segment behind
    /// the acknowledged checkpoint (safety/replay window).
    pub prune_enabled: bool,
    /// Batch row bound.
    pub batch_rows: usize,
    /// Batch byte bound.
    pub batch_bytes: usize,
    /// Address the batch-ack latency `/metrics` endpoint binds to.
    pub metrics_addr: String,
}

impl Default for IngesterConfig {
    fn default() -> Self {
        Self {
            clickhouse_url: "http://127.0.0.1:8123".into(),
            database: "market".into(),
            user: "default".into(),
            password: String::new(),
            catalog_path: "catalog.db".into(),
            checkpoint_path: "checkpoints.db".into(),
            replay_budget_bytes: 64 * 1024 * 1024,
            archive_segment_length: 0,
            prune_enabled: false,
            batch_rows: ergo_clickhouse_persist::protocol::limits::INGEST_BATCH_ROWS,
            batch_bytes: ergo_clickhouse_persist::protocol::limits::INGEST_BATCH_BYTES,
            metrics_addr: "0.0.0.0:9102".into(),
        }
    }
}

/// Per-source pipeline state.
struct SourceState {
    id: String,
    /// Key this source's checkpoint is stored under.
    ///
    /// Starts as the source id, and becomes `<id>:<recording>` once a
    /// recording is resolved. PLAN §5 requires "the greatest contiguous
    /// acknowledged prefix **per recording**": keyed by source alone, a new
    /// recording resumed at the *previous* recording's offset, which is past
    /// its data, so every frame was treated as already-consumed.
    checkpoint_key: String,
    run_id: u64,
    prefix: ContiguousPrefix,
    batches: std::collections::HashMap<u16, BatchBuffer>,
    /// Layouts seen (resolved against the durable catalog).
    layouts: std::collections::HashMap<u16, Binding>,
    schemas: std::collections::HashMap<u16, ergo_clickhouse_persist::schema::RowSchema>,
    /// Temporary tables' lifecycle, seeded from and written back to the
    /// durable journal. Only temporary layouts appear here.
    lifecycle: std::collections::HashMap<u16, LifecycleRecord>,
    /// Recording descriptor once resolved (for pruning).
    #[cfg(feature = "archive")]
    recording: Option<ergo_clickhouse_persist::ingest::archive::RecordingInfo>,
    /// This source's archive client.
    ///
    /// Per source, not per ingester: PLAN §9 wants every recorder's Archive
    /// read, and each is a separate control endpoint. This used to be a single
    /// field on `Ingester`, which is exactly why only one recorder was ever
    /// ingested.
    #[cfg(feature = "archive")]
    archive_client: Option<ergo_clickhouse_persist::ingest::archive::ArchiveClient>,
}

/// Fixed-bucket Prometheus-style histogram for ClickHouse batch send-to-
/// acknowledgement latency (PLAN §10: "ClickHouse batch-acknowledgement
/// histograms").
///
/// Fires once per flushed batch — rows (8192), bytes (8 MiB), or 100 ms,
/// whichever comes first — never on the per-record path, so a plain atomic
/// bump per observation needs no sampling. Bucket bounds are seconds, per
/// Prometheus's own histogram guidance (base units, not milliseconds).
struct BatchAckHistogram {
    buckets: [AtomicU64; Self::BOUNDS_SECONDS.len()],
    count: AtomicU64,
    sum_micros: AtomicU64,
}

impl BatchAckHistogram {
    const BOUNDS_SECONDS: [f64; 12] = [
        0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
    ];

    fn new() -> Self {
        Self {
            buckets: std::array::from_fn(|_| AtomicU64::new(0)),
            count: AtomicU64::new(0),
            sum_micros: AtomicU64::new(0),
        }
    }

    /// Record one observation from a monotonic-clock duration.
    fn observe(&self, elapsed: std::time::Duration) {
        let secs = elapsed.as_secs_f64();
        for (bucket, bound) in self.buckets.iter().zip(Self::BOUNDS_SECONDS) {
            if secs <= bound {
                bucket.fetch_add(1, Ordering::Relaxed);
            }
        }
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum_micros
            .fetch_add(elapsed.as_micros() as u64, Ordering::Relaxed);
    }

    fn render(&self) -> String {
        let mut out = String::from(
            "# HELP ergo_ingester_batch_ack_seconds ClickHouse batch insert \
             send-to-acknowledgement latency\n\
             # TYPE ergo_ingester_batch_ack_seconds histogram\n",
        );
        for (bucket, bound) in self.buckets.iter().zip(Self::BOUNDS_SECONDS) {
            let c = bucket.load(Ordering::Relaxed);
            out.push_str(&format!(
                "ergo_ingester_batch_ack_seconds_bucket{{le=\"{bound}\"}} {c}\n"
            ));
        }
        let total = self.count.load(Ordering::Relaxed);
        out.push_str(&format!(
            "ergo_ingester_batch_ack_seconds_bucket{{le=\"+Inf\"}} {total}\n"
        ));
        let sum_s = self.sum_micros.load(Ordering::Relaxed) as f64 / 1_000_000.0;
        out.push_str(&format!("ergo_ingester_batch_ack_seconds_sum {sum_s}\n"));
        out.push_str(&format!("ergo_ingester_batch_ack_seconds_count {total}\n"));
        out
    }
}

/// Serve `/metrics` for the batch-ack histogram on a blocking loop — the
/// same minimal stdlib-only pattern as `archive-agent`'s `serve_metrics`,
/// kept separate rather than shared because this histogram is specific to
/// the ingester's own batch-flush path, not a registration counter.
fn serve_batch_latency_metrics(
    addr: &str,
    histogram: Arc<BatchAckHistogram>,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let mut buf = [0u8; 1024];
        let _ = stream.read(&mut buf);
        let body = histogram.render();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
    }
    Ok(())
}

/// The ingester over one or more sources.
pub struct Ingester {
    config: IngesterConfig,
    ch: ClickHouse,
    catalog: Catalog,
    checkpoints: CheckpointStore,
    /// Schema conflicts counted during reconciliation.
    pub schema_conflicts: u64,
    /// Rows ingested.
    pub rows_ingested: u64,
    /// ClickHouse batch send-to-acknowledgement latency (PLAN §10).
    latency: Arc<BatchAckHistogram>,
    sources: Vec<SourceState>,
    /// When the next background cleanup pass may run.
    next_lifecycle_pass: std::time::Instant,
}

/// How often temporary-table cleanup runs. The idle windows it enforces are
/// measured in hours, so this only needs to be prompt, not precise.
const LIFECYCLE_PASS_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

impl Ingester {
    /// New ingester; opens the durable catalog and checkpoint stores.
    pub fn open(config: IngesterConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let ch = ClickHouse::new(
            &config.clickhouse_url,
            &config.database,
            &config.user,
            &config.password,
        );
        let catalog = Catalog::open(&config.catalog_path)?;
        let checkpoints = CheckpointStore::open(&config.checkpoint_path)?;
        let bootstrap = ClickHouse::new(
            &config.clickhouse_url,
            "default",
            &config.user,
            &config.password,
        );
        bootstrap.exec(&format!(
            "CREATE DATABASE IF NOT EXISTS `{}`",
            config.database
        ))?;
        // Archive position/health samples. Written by the Aeron replay leg;
        // the table exists in every mode so a notebook or panel query is
        // never a missing-table error.
        bootstrap.exec(&format!(
            "CREATE TABLE IF NOT EXISTS `{}`.`aeron_metrics` (\
               captured_at_ns UInt64, run_id UInt64, recording_id Int64, \
               recording_start_position Int64, recording_stop_position Int64, \
               replay_position Int64, rows_ingested UInt64\
             ) ENGINE = MergeTree ORDER BY (captured_at_ns)",
            config.database
        ))?;
        // Always-present status table: a temporary table that was never
        // enabled has no data and no view rows, so dashboards and notebooks
        // read its applied policy from here instead of hitting a missing
        // table. Keyed per applied layout and collapsed by ReplacingMergeTree.
        bootstrap.exec(&format!(
            "CREATE TABLE IF NOT EXISTS `{}`.`_recording_status` (\
               table_name String, layout_id UInt16, policy String, \
               row_ttl_ns UInt64, idle_ttl_ns UInt64, projection_revision UInt32, \
               run_id UInt64, declared_at_ns UInt64, updated_at_ns UInt64\
             ) ENGINE = ReplacingMergeTree(updated_at_ns) ORDER BY (table_name, layout_id)",
            config.database
        ))?;
        Ok(Self {
            config,
            ch,
            catalog,
            checkpoints,
            sources: Vec::new(),
            latency: Arc::new(BatchAckHistogram::new()),
            next_lifecycle_pass: std::time::Instant::now(),
            schema_conflicts: 0,
            rows_ingested: 0,
        })
    }

    /// Register a source; resumes at its persisted checkpoint if present.
    pub fn add_source(&mut self, id: &str, run_id: u64) -> Result<(), Box<dyn std::error::Error>> {
        let prefix = ContiguousPrefix {
            committed: self.checkpoints.get(id).unwrap_or(0),
            pending: Vec::new(),
        };
        self.sources.push(SourceState {
            id: id.to_string(),
            checkpoint_key: id.to_string(),
            run_id,
            prefix,
            batches: std::collections::HashMap::new(),
            layouts: std::collections::HashMap::new(),
            schemas: std::collections::HashMap::new(),
            lifecycle: std::collections::HashMap::new(),
            #[cfg(feature = "archive")]
            recording: None,
            #[cfg(feature = "archive")]
            archive_client: None,
        });
        Ok(())
    }

    /// How many sources are registered.
    #[must_use]
    pub fn source_count(&self) -> usize {
        self.sources.len()
    }

    /// Persisted checkpoint for a source id, or 0 when it has none.
    #[must_use]
    pub fn resume_position(&self, id: &str) -> i64 {
        self.checkpoints.get(id).unwrap_or(0)
    }

    /// Point a source at a recording and return where it should resume from.
    ///
    /// PLAN §5: "Maintain the greatest contiguous acknowledged prefix **per
    /// recording**: if later batches complete before an earlier one, do not
    /// advance past the earlier gap." The checkpoint used to be keyed by
    /// source alone, so a *new* recording resumed at the previous recording's
    /// offset — beyond its data — and every frame of it was skipped. Observed
    /// live: a recorder restart published a new recording and the ingester
    /// ingested **zero** new rows.
    ///
    /// Idempotent for a recording already being read: it returns the committed
    /// position and leaves the in-memory prefix (including any pending gaps)
    /// alone, so re-resolving on every pass cannot discard progress.
    pub fn retarget_recording(&mut self, source_idx: usize, recording_id: i64) -> i64 {
        let key = format!("{}:{recording_id}", self.sources[source_idx].id);
        if self.sources[source_idx].checkpoint_key == key {
            return self.sources[source_idx].prefix.committed;
        }
        let committed = self.checkpoints.get(&key).unwrap_or(0);
        let state = &mut self.sources[source_idx];
        state.checkpoint_key = key;
        state.prefix = ContiguousPrefix {
            committed,
            pending: Vec::new(),
        };
        committed
    }

    /// Process one batch from a source: declarations resolve against the
    /// durable catalog; data records project into batch buffers; flushes
    /// push one binary insert; checkpoints advance only on acknowledged
    /// contiguous prefixes. After a checkpoint commits, the source's
    /// archive segments below the safety window are pruned (active
    /// recordings via purge, stopped via truncate).
    pub fn process_batch(
        &mut self,
        source_idx: usize,
        batch: &ergo_clickhouse_persist::ingest::SourceBatch,
    ) -> Result<(), SourceError> {
        for item in &batch.items {
            match item {
                SourceItem::Declaration(decl) => self.apply_declaration(source_idx, decl),
                SourceItem::Record(env) => self.project_record(source_idx, env)?,
            }
        }
        // Contiguous-prefix tracking on batch completion; the checkpoint
        // commit is the pruning trigger (prune only acknowledged data).
        let state = &mut self.sources[source_idx];
        if let Some(committed) = state
            .prefix
            .complete(batch.start_position, batch.end_position)
        {
            self.checkpoints
                .put(&state.checkpoint_key, committed)
                .map_err(|e| SourceError::InvalidDeclaration(format!("checkpoint: {e}")))?;
            self.prune_source(source_idx, committed);
        }
        Ok(())
    }

    /// Prune the source's archive recording behind the acknowledged
    /// checkpoint. No-op without an archive client or when pruning is
    /// disabled; never errors the source (pruning is best-effort and the
    /// checkpoint is already durable).
    fn prune_source(&mut self, source_idx: usize, committed: i64) {
        #[cfg(feature = "archive")]
        if self.config.prune_enabled
            && self.config.archive_segment_length > 0
            && let (Some(client), Some(info)) = (
                &self.sources[source_idx].archive_client,
                self.sources[source_idx].recording,
            )
        {
            match ergo_clickhouse_persist::ingest::archive::prune_after_checkpoint(
                client,
                &info,
                committed,
                self.config.archive_segment_length,
            ) {
                Ok(segments) if segments > 0 => {
                    eprintln!(
                        "ingester: pruned {segments} segment(s) below checkpoint {committed}"
                    );
                }
                Ok(_) => {}
                Err(e) => eprintln!("ingester: prune deferred: {e}"),
            }
        }
        #[cfg(not(feature = "archive"))]
        let _ = (source_idx, committed);
    }

    /// Attach the archive client used for post-checkpoint pruning and
    /// resolve the recording for a source.
    #[cfg(feature = "archive")]
    pub fn attach_archive(
        &mut self,
        client: ergo_clickhouse_persist::ingest::archive::ArchiveClient,
        source_idx: usize,
        info: ergo_clickhouse_persist::ingest::archive::RecordingInfo,
    ) {
        self.sources[source_idx].archive_client = Some(client);
        self.sources[source_idx].recording = Some(info);
    }

    /// The attached archive client, for re-resolving recordings.
    ///
    /// A long-running ingester has to keep asking the archive what it should
    /// be reading, and after [`Self::attach_archive`] the client lives here
    /// rather than in the caller. `ArchiveReplaySource` does not borrow it, so
    /// this stays usable between passes.
    #[cfg(feature = "archive")]
    #[must_use]
    pub fn archive_client(
        &self,
        source_idx: usize,
    ) -> Option<&ergo_clickhouse_persist::ingest::archive::ArchiveClient> {
        self.sources[source_idx].archive_client.as_ref()
    }

    /// Point a source at a recording without re-attaching the client.
    #[cfg(feature = "archive")]
    pub fn set_recording(
        &mut self,
        source_idx: usize,
        info: ergo_clickhouse_persist::ingest::archive::RecordingInfo,
    ) {
        self.sources[source_idx].recording = Some(info);
    }

    /// Append one archive/replay sample to `aeron_metrics`.
    ///
    /// Publishes `_recording_` positions, which is what the archive can
    /// actually report; the publisher's own position is never read back, so
    /// it is not invented here.
    #[cfg(feature = "archive")]
    pub fn sample_aeron_metrics(&self, source_idx: usize, replay_position: i64) {
        let Some(client) = self.sources[source_idx].archive_client.as_ref() else {
            return;
        };
        let Some(recording) = self.sources[source_idx].recording.as_ref() else {
            return;
        };
        let (start, stop) = client
            .current()
            .map_or((recording.start_position, recording.stop_position), |r| {
                (r.start_position, r.stop_position)
            });
        let mut body = Vec::with_capacity(64);
        body.extend_from_slice(&ergo_clickhouse_persist::registration::now_ns().to_le_bytes());
        body.extend_from_slice(&self.sources[source_idx].run_id.to_le_bytes());
        body.extend_from_slice(&recording.recording_id.to_le_bytes());
        body.extend_from_slice(&start.to_le_bytes());
        body.extend_from_slice(&stop.to_le_bytes());
        body.extend_from_slice(&replay_position.to_le_bytes());
        body.extend_from_slice(&self.rows_ingested.to_le_bytes());
        let columns: Vec<String> = [
            "captured_at_ns",
            "run_id",
            "recording_id",
            "recording_start_position",
            "recording_stop_position",
            "replay_position",
            "rows_ingested",
        ]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
        let table = format!("`{}`.`aeron_metrics`", self.config.database);
        if let Err(e) = self.ch.insert_raw(&table, &columns, &body) {
            eprintln!("ingester: aeron metric sample failed: {e:?}");
        }
    }

    /// Apply a declaration to **the source that delivered it**.
    ///
    /// `source_idx` is not decoration: this used to write layouts and schemas
    /// to `self.sources.last_mut()`, which is that same source only while
    /// there is one. With two (PLAN §9), declarations from the first Archive
    /// landed on the second source's state, so the first source's records
    /// found no layout and the run died with "record for unknown layout 3".
    /// One source hid it completely.
    fn apply_declaration(&mut self, source_idx: usize, decl: &Declaration) {
        match decl {
            Declaration::SessionStart(s) => {
                let _ = self.catalog.put_session(
                    s.run_id,
                    &s.process,
                    &s.instance,
                    &s.build,
                    s.started_at_ns,
                );
            }
            Declaration::Policy(p) => {
                let _ = self.catalog.put_policy(self.sources[source_idx].run_id, p);
            }
            Declaration::Layout(l) => {
                self.ensure_backing_table(source_idx, l);
            }
            _ => {}
        }
    }

    /// Ensure the backing table + public view exist and the frozen binding
    /// is persisted before any data is accepted.
    fn ensure_backing_table(&mut self, source_idx: usize, l: &LayoutDeclaration) {
        let run_id = self.sources[source_idx].run_id;
        // Build the storage schema first: a re-declared layout id must be
        // checked against it before being discarded. The DDL is `CREATE TABLE
        // IF NOT EXISTS`, so an incompatible re-declaration would otherwise be
        // swallowed silently and every later row projected against the wrong
        // column types.
        let schema = ergo_clickhouse_persist::schema::RowSchema::from_vec(
            l.columns
                .iter()
                .map(|c| ergo_clickhouse_persist::schema::ValueSchema {
                    ty: num_to_type(c.type_code),
                    precision: c.precision,
                    scale: c.scale,
                    nullable: c.flags & 1 != 0,
                    is_array: c.flags & 2 != 0,
                })
                .collect(),
        );
        if let Some(bound) = self.sources[source_idx].schemas.get(&l.layout_id) {
            if bound.fingerprint() != schema.fingerprint() {
                self.schema_conflicts += 1;
                eprintln!(
                    "ingester: layout {} ({}) re-declared with a different schema; \
                     keeping the first binding and counting the conflict",
                    l.layout_id, l.table_name
                );
            }
            return;
        }
        let backing = format!("ingest_{}_l{}", sanitize(&l.table_name), l.layout_id);
        let view = l.table_name.clone();
        let mut column_names: Vec<String> = l.columns.iter().map(|c| c.name.clone()).collect();
        let sys_cols = [
            "_record_captured_at_ns",
            "_record_run_id",
            "_record_writer_id",
            "_record_sequence",
            "_record_row_index",
        ];
        column_names.extend(sys_cols.iter().map(|s| (*s).to_string()));
        if l.payload_encoding == RowEncoding::DynamicRow
            || l.payload_encoding == RowEncoding::OrderedRow
        {
            column_names.push("_record_row_ttl_ns".to_string());
        }
        let policy = self.catalog.policy(run_id, l.policy_id);
        let temporary = policy
            .as_ref()
            .is_some_and(|p| p.policy == Policy::Temporary);
        // PLAN §5: a row's expiry is computed from its capture time and *its*
        // policy, so the effective retention is resolved once here and frozen
        // onto the binding. It was previously not recorded at all, which is
        // why every temporary row was written with a zero TTL.
        let row_ttl_ns = policy.as_ref().map_or(0, |p| p.row_ttl_ns);
        // The idle window is frozen alongside the retention for the same
        // reason: cleanup must not become eligible at a different moment
        // because the config changed after the table was bound.
        let idle_ttl_ns = policy.as_ref().map_or(0, |p| p.idle_ttl_ns);

        let ddl = create_table_ddl(&backing, &schema, &column_names, temporary, &[]);
        if let Err(e) = self.ch.exec(&ddl) {
            eprintln!("ingester: DDL failed for {backing}: {e:?}");
            return; // failed DDL must not mark the column as installed
        }
        let view_ddl = create_view_ddl(
            &format!("`{}`.`{view}`", self.config.database),
            &format!("`{}`.`{backing}`", self.config.database),
            temporary,
        );
        if let Err(e) = self.ch.exec(&view_ddl) {
            eprintln!("ingester: view DDL failed for {view}: {e:?}");
            return;
        }
        let binding = Binding {
            run_id,
            layout_id: l.layout_id,
            table: l.table_name.clone(),
            backing,
            view,
            columns: column_names,
            omitted: Vec::new(),
            projection_revision: l.projection_revision,
            temporary,
            row_ttl_ns,
            idle_ttl_ns,
        };
        let _ = self.catalog.put_binding(&binding);
        self.publish_status(&binding, temporary, l.policy_id);
        {
            let state = &mut self.sources[source_idx];
            if temporary {
                // Resume an existing generation, or open the first one. The
                // record is written immediately so a crash before any row
                // arrives still leaves cleanup with something to act on.
                //
                // The idle clock starts when the generation does: seeding it
                // with 0 instead reads as "idle since 1970", which drops the
                // table on the very first cleanup pass. PLAN §5's seven-day
                // idle default is measured from here, not from the epoch.
                let created_at = ergo_clickhouse_persist::registration::now_ns();
                let record =
                    self.catalog
                        .lifecycle(run_id, l.layout_id)
                        .unwrap_or(LifecycleRecord {
                            run_id,
                            layout_id: l.layout_id,
                            table: binding.table.clone(),
                            generation: 1,
                            state: LifecycleState::Active,
                            latest_expiry_ns: created_at,
                            last_input_ns: created_at,
                            view_dropped: false,
                            backing_dropped: false,
                        });
                let _ = self.catalog.put_lifecycle(&record);
                state.lifecycle.insert(l.layout_id, record);
            }
            state.layouts.insert(l.layout_id, binding);
            state.schemas.insert(l.layout_id, schema);
        }
    }

    /// Record one applied layout's effective policy in `_recording_status`.
    ///
    /// This is the only surface that names a declared-but-never-enabled
    /// temporary table: such a table has no view rows and no data.
    fn publish_status(&self, binding: &Binding, temporary: bool, policy_id: u16) {
        use ergo_clickhouse_persist::ingest::clickhouse::bin;
        let declared = self
            .catalog
            .policy(binding.run_id, policy_id)
            .map_or((0, 0), |p| (p.row_ttl_ns, p.idle_ttl_ns));
        let now = ergo_clickhouse_persist::registration::now_ns();
        let mut body = Vec::with_capacity(128);
        bin::string(&mut body, binding.table.as_bytes());
        body.extend_from_slice(&binding.layout_id.to_le_bytes());
        bin::string(
            &mut body,
            if temporary {
                b"temporary"
            } else {
                b"permanent"
            },
        );
        body.extend_from_slice(&declared.0.to_le_bytes());
        body.extend_from_slice(&declared.1.to_le_bytes());
        body.extend_from_slice(&binding.projection_revision.to_le_bytes());
        body.extend_from_slice(&binding.run_id.to_le_bytes());
        body.extend_from_slice(&now.to_le_bytes());
        body.extend_from_slice(&now.to_le_bytes());
        let columns: Vec<String> = [
            "table_name",
            "layout_id",
            "policy",
            "row_ttl_ns",
            "idle_ttl_ns",
            "projection_revision",
            "run_id",
            "declared_at_ns",
            "updated_at_ns",
        ]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
        let table = format!("`{}`.`_recording_status`", self.config.database);
        if let Err(e) = self.ch.insert_raw(&table, &columns, &body) {
            eprintln!("ingester: status row for {} failed: {e:?}", binding.table);
        }
    }

    fn project_record(
        &mut self,
        source_idx: usize,
        env: &ergo_clickhouse_persist::protocol::DataEnvelope,
    ) -> Result<(), SourceError> {
        let state = &mut self.sources[source_idx];
        let Some(binding) = state.layouts.get(&env.layout_id).cloned() else {
            return Err(SourceError::InvalidDeclaration(format!(
                "record for unknown layout {} at seq {}",
                env.layout_id, env.metadata.sequence
            )));
        };
        let Some(schema) = state.schemas.get(&env.layout_id).cloned() else {
            return Err(SourceError::InvalidDeclaration(
                "layout schema missing".into(),
            ));
        };
        let include_ttl = binding.columns.iter().any(|c| c == "_record_row_ttl_ns");
        // The row's expiry comes from the binding's frozen policy retention,
        // not from a literal: writing `0` here gave every temporary row a TTL
        // of the epoch, so the row was expired the moment it was captured.
        let body = ergo_clickhouse_persist::ingest::clickhouse::persist_payload_to_rowbinary(
            &schema,
            &env.payload,
            &env.metadata,
            state.run_id,
            include_ttl,
            binding.row_ttl_ns,
        )
        .map_err(|_| SourceError::Malformed("rowbinary convert"))?;
        let flush = {
            // The configured bounds are applied here; `or_default()` used the
            // protocol defaults, so `--batch-rows`/`--batch-bytes` were
            // accepted and then ignored.
            let buf = state.batches.entry(env.layout_id).or_insert_with(|| {
                BatchBuffer::with_limits(self.config.batch_rows, self.config.batch_bytes)
            });
            buf.push_row(std::iter::once(body));
            buf.should_flush()
        };
        if flush {
            self.flush_layout(source_idx, env.layout_id)?;
        }
        // Keep the temporary table's clocks honest. Cleanup decides on these
        // facts, so they come from the record's own capture time rather than
        // from wall-clock guesses. Updated in memory only: journalling per row
        // would put a SQLite write on the hot path, so the periodic pass is
        // what persists them.
        // Re-borrowed rather than reusing the binding above: the flush in
        // between takes `&mut self`, so the earlier borrow has to be dead here.
        let state = &mut self.sources[source_idx];
        if let Some(record) = state.lifecycle.get_mut(&env.layout_id) {
            let now = ergo_clickhouse_persist::registration::now_ns();
            let expiry = row_expiry_ns(env.metadata.captured_at_ns, binding.row_ttl_ns, now);
            record.latest_expiry_ns = record.latest_expiry_ns.max(expiry);
            record.last_input_ns = now;
            // PLAN §5: fresh input cancels a drop that began but has not yet
            // removed anything.
            if on_input(record.state, false) == OnInput::CancelDrop {
                record.state = LifecycleState::Active;
            }
        }
        Ok(())
    }

    /// Background cleanup for temporary tables.
    ///
    /// PLAN §5: a table is dropped only once all retained rows have expired,
    /// its idle window has passed, and nothing is still writing to it. Every
    /// decision goes through [`may_drop`] and every removal through
    /// [`advance_drop`], so the ingester holds no separate copy of the policy.
    ///
    /// A live session is represented by the idle window rather than by a
    /// separate registration count: a table that is still receiving input is
    /// never idle, and one that is not has no registration left to honour.
    /// "Pending writes" is therefore the only extra gate, and it is the real
    /// one — unflushed rows that a drop would discard.
    pub fn lifecycle_pass(&mut self, now_ns: u64) {
        for idx in 0..self.sources.len() {
            let layout_ids: Vec<u16> = self.sources[idx].lifecycle.keys().copied().collect();
            for layout_id in layout_ids {
                let Some(binding) = self.sources[idx].layouts.get(&layout_id).cloned() else {
                    continue;
                };
                let Some(mut record) = self.sources[idx].lifecycle.get(&layout_id).cloned() else {
                    continue;
                };
                let pending = u64::try_from(
                    self.sources[idx]
                        .batches
                        .get(&layout_id)
                        .map_or(0, |b| b.rows()),
                )
                .unwrap_or(0);
                // A zero window is the wire's spelling for "no policy"
                // (`row_ttl_ns`/`idle_ttl_ns` are documented as
                // "0 = permanent/default"), *not* for "expired immediately".
                // Reading it as an elapsed window would make cleanup drop any
                // temporary table whose producer declared no retention — while
                // it was still being written to. So a table with no idle policy
                // is never eligible.
                if !has_idle_policy(binding.idle_ttl_ns) {
                    continue;
                }
                // A record whose clock never started — written by an earlier
                // build, or restored from a journal made before the clock was
                // seeded — must not read as ancient history.
                if !idle_clock_started(record.last_input_ns) {
                    continue;
                }
                let idle_deadline = record.last_input_ns.saturating_add(binding.idle_ttl_ns);

                let mut changed = false;
                if record.state == LifecycleState::Active
                    && may_drop(record.latest_expiry_ns, idle_deadline, now_ns, pending, 0).is_ok()
                {
                    record.state = LifecycleState::DropPending;
                    changed = true;
                    eprintln!(
                        "ingester: temporary table {} is idle and expired; dropping",
                        record.table
                    );
                }
                if record.state == LifecycleState::DropPending {
                    match advance_drop(
                        &self.ch,
                        &self.catalog,
                        &mut record,
                        &binding.view,
                        &binding.backing,
                    ) {
                        Ok(()) => {
                            changed = true;
                            if record.state == LifecycleState::Dropped {
                                eprintln!(
                                    "ingester: dropped temporary table {} (generation {})",
                                    record.table, record.generation
                                );
                            }
                        }
                        Err(e) => eprintln!(
                            "ingester: dropping {} failed, will retry: {e:?}",
                            record.table
                        ),
                    }
                }
                if changed {
                    let _ = self.catalog.put_lifecycle(&record);
                    self.sources[idx].lifecycle.insert(layout_id, record);
                }
            }
        }
    }

    /// Run cleanup at most once per interval.
    ///
    /// Cleanup is background work; running the pass per flush would put DDL and
    /// journal writes on the ingest path that this crate keeps allocation- and
    /// IO-free.
    fn maybe_lifecycle_pass(&mut self) {
        let now = std::time::Instant::now();
        if now < self.next_lifecycle_pass {
            return;
        }
        self.next_lifecycle_pass = now + LIFECYCLE_PASS_INTERVAL;
        self.lifecycle_pass(ergo_clickhouse_persist::registration::now_ns());
    }

    fn flush_layout(&mut self, source_idx: usize, layout_id: u16) -> Result<(), SourceError> {
        let (table, columns, body_bytes, rows) = {
            let state = &mut self.sources[source_idx];
            let Some(binding) = state.layouts.get(&layout_id) else {
                return Ok(());
            };
            let Some(buf) = state.batches.get_mut(&layout_id) else {
                return Ok(());
            };
            if buf.rows() == 0 {
                return Ok(());
            }
            let rows = buf.rows();
            let body_bytes = buf.body().to_vec();
            let columns = binding.columns.clone();
            let table = format!("`{}`", binding.backing);
            buf.clear();
            (table, columns, body_bytes, rows)
        };
        let started = Instant::now();
        let result = self.ch.insert_raw(&table, &columns, &body_bytes);
        // Recorded on both outcomes: a request that stalls then fails is
        // exactly the "drop" signal task 9 wants visible, not a sample to
        // discard.
        self.latency.observe(started.elapsed());
        result.map_err(|e| SourceError::Disconnected(e.to_string()))?;
        self.rows_ingested += u64::try_from(rows).unwrap_or(0);
        Ok(())
    }

    /// Flush all pending batches (one RowBinary insert per layout).
    /// Per-table inserts are driven by the projection wiring; batch bounds
    /// (rows/bytes/time) are exercised by the library tests.
    pub fn flush(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        let mut flushed = 0;
        let jobs: Vec<(usize, u16, usize)> = self
            .sources
            .iter()
            .enumerate()
            .flat_map(|(i, s)| {
                s.batches
                    .iter()
                    .filter(|(_, b)| b.rows() > 0)
                    .map(move |(layout, b)| (i, *layout, b.rows()))
                    .collect::<Vec<_>>()
            })
            .collect();
        for (i, layout, rows) in jobs {
            self.flush_layout(i, layout)?;
            flushed += rows;
        }
        Ok(flushed)
    }
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn num_to_type(code: u8) -> ergo_clickhouse_persist::schema::TypeCode {
    match code {
        1 => ergo_clickhouse_persist::schema::TypeCode::Bool,
        2 => ergo_clickhouse_persist::schema::TypeCode::I8,
        3 => ergo_clickhouse_persist::schema::TypeCode::I16,
        4 => ergo_clickhouse_persist::schema::TypeCode::I32,
        5 => ergo_clickhouse_persist::schema::TypeCode::I64,
        6 => ergo_clickhouse_persist::schema::TypeCode::U8,
        7 => ergo_clickhouse_persist::schema::TypeCode::U16,
        8 => ergo_clickhouse_persist::schema::TypeCode::U32,
        9 => ergo_clickhouse_persist::schema::TypeCode::U64,
        10 => ergo_clickhouse_persist::schema::TypeCode::F32,
        11 => ergo_clickhouse_persist::schema::TypeCode::F64,
        12 => ergo_clickhouse_persist::schema::TypeCode::Decimal,
        13 => ergo_clickhouse_persist::schema::TypeCode::Utf8,
        14 => ergo_clickhouse_persist::schema::TypeCode::Bytes,
        _ => ergo_clickhouse_persist::schema::TypeCode::TimestampNs,
    }
}

// `LabSource` used to live here: a single-threaded ring adapter that was never
// constructed anywhere (`IngestSource` is implemented by the library's
// `MemorySource`, which the lab tests use) and whose scratch buffer was sized
// `MAX_RECORD_BYTES` while its drain loop accumulated up to `budget.bytes / 2`
// — an index panic on any run past 1 MiB. Dead *and* broken is not worth
// repairing; `drained_len`/`share`/`poll_interval` went with it.

/// Aeron replay options (mode `aeron`).
#[cfg(feature = "archive")]
impl Default for AeronOptions {
    fn default() -> Self {
        let jar = std::env::var("ERGO_AERON_JAR").unwrap_or_else(|_| {
            let home = std::env::home_dir().unwrap_or_else(std::env::temp_dir);
            home.join(".cache/ergo/aeron/aeron-all-1.53.2.jar")
                .display()
                .to_string()
        });
        Self {
            archive_jar: jar,
            archive_base: "target/ingester-archive".into(),
            aeron_dir: String::new(),
            stream_id: 42,
            segment_length: 64 * 1024 * 1024,
            replay_channel: None,
            follow: false,
            // A fixed default collided every re-run against a persistent
            // database: `_record_run_id` stays part of ReplacingMergeTree's
            // dedup key, so two ingests of the same archive under the same
            // run_id produce distinct rows (captured_at differs) that are
            // nonetheless the same logical event replayed twice.
            // `--run-id` still overrides this for anyone who wants a stable,
            // reproducible identity.
            run_id: ergo_clickhouse_persist::registration::fresh_run_id(),
            publish_export: None,
            archive_sources: Vec::new(),
        }
    }
}

/// Apply one `--archive-*`/`--aeron-*` flag to the Aeron options.
#[cfg(feature = "archive")]
fn apply_aeron_arg(
    opts: &mut AeronOptions,
    flag: &str,
    value: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let v = value.unwrap_or_default();
    match flag {
        "--archive-jar" => opts.archive_jar = v.into(),
        "--archive-base" => opts.archive_base = v.into(),
        "--aeron-dir" => opts.aeron_dir = v.into(),
        "--replay-channel" => opts.replay_channel = Some(v.into()),
        "--stream-id" => opts.stream_id = v.parse()?,
        "--segment-length" => opts.segment_length = v.parse()?,
        "--run-id" => opts.run_id = v.parse()?,
        "--publish-export" => opts.publish_export = Some(v.into()),
        // Repeatable: one per recorder whose Archive should be read.
        "--archive-source" => opts.archive_sources.push(v.into()),
        "--follow" => opts.follow = true,
        other => {
            eprintln!("unknown argument: {other}");
            return Err("bad arguments".into());
        }
    }
    Ok(())
}

/// Publish a length-prefixed export file into the archive's recording
/// channel, mirroring the producer pod's transport path. Returns the number
/// of frames offered and the archive recording subscription to stop.
#[cfg(feature = "archive")]
fn publish_export_into_aeron(
    endpoints: &ergo_clickhouse_persist::ingest::archive::ArchiveEndpoints,
    aeron_dir: &std::path::Path,
    archive: &rusteron_archive::AeronArchive,
    path: &str,
    stream_id: i32,
) -> Result<(usize, i64), Box<dyn std::error::Error>> {
    let raw = std::fs::read(path)?;
    let mut frames: Vec<Vec<u8>> = Vec::new();
    let mut off = 0usize;
    while off + 4 <= raw.len() {
        let len = u32::from_le_bytes(raw[off..off + 4].try_into()?) as usize;
        off += 4;
        if off + len > raw.len() {
            return Err(format!("truncated export frame at byte {off}").into());
        }
        frames.push(raw[off..off + len].to_vec());
        off += len;
    }
    // The recording subscription must exist before the publication connects,
    // otherwise the offers are lost to a channel nobody is recording.
    let subscription_id = archive.start_recording(
        &rusteron_archive::cformat!("{}", endpoints.recorded_channel),
        stream_id,
        rusteron_archive::SOURCE_LOCATION_LOCAL,
        true,
    )?;

    let ctx = rusteron_client::AeronContext::new()?;
    ctx.set_dir(&rusteron_client::cformat!("{}", aeron_dir.display()))?;
    let aeron = rusteron_client::Aeron::new(&ctx)?;
    aeron.start()?;
    let publication = aeron.add_exclusive_publication(
        &rusteron_client::cformat!("{}", endpoints.recorded_channel),
        stream_id,
        Duration::from_secs(30),
    )?;
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while !publication.is_connected() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    if !publication.is_connected() {
        return Err("archive recording publication never connected".into());
    }

    let mut sent = 0usize;
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while sent < frames.len() && std::time::Instant::now() < deadline {
        if publication.offer_raw(&frames[sent], rusteron_client::Handlers::NONE) > 0 {
            sent += 1;
        } else {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    if sent != frames.len() {
        return Err(format!("published {sent} of {} frames", frames.len()).into());
    }
    // Let the archiver drain before the subscription is stopped.
    std::thread::sleep(Duration::from_millis(1500));
    Ok((sent, subscription_id))
}

/// Aeron replay options (mode `aeron`).
#[cfg(feature = "archive")]
#[derive(Clone, Debug)]
struct AeronOptions {
    archive_jar: String,
    archive_base: String,
    aeron_dir: String,
    stream_id: i32,
    segment_length: usize,
    replay_channel: Option<String>,
    follow: bool,
    run_id: u64,
    /// Export file to publish into Aeron before replaying it back.
    publish_export: Option<String>,
    /// Recorders whose Archives to read, by name.
    ///
    /// Each name resolves that recorder's endpoints through its headless
    /// Service (`<name>-archive.<ns>.svc.cluster.local`) — PLAN §9's "resolve
    /// each recorder's endpoints through its headless Service and iterate
    /// sources". Empty means the single env-configured Archive, which is what
    /// the lab and CI paths use. Every recorder pod owns its own Archive, so a
    /// single-source ingester reads one exchange and silently ignores the
    /// rest; the cluster runs `binance-archive` and `bybit-archive`.
    archive_sources: Vec<String>,
}

#[cfg(feature = "archive")]
fn ingest_aeron(
    ingester: &mut Ingester,
    opts: &AeronOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    use ergo_clickhouse_persist::ingest::archive::{
        ArchiveClient, ArchiveEndpoints, ArchiveServer,
    };

    // Pruning aligns to a segment boundary, so it needs the segment length the
    // archive was launched with. `--segment-length` is the only source of that
    // value, and it previously never reached the config — leaving `--prune` a
    // documented no-op.
    ingester.config.archive_segment_length = opts.segment_length;

    let base = std::path::PathBuf::from(&opts.archive_base);
    let launch_local = opts.aeron_dir.is_empty();
    let server = if launch_local {
        Some(ArchiveServer::launch(
            &opts.archive_jar,
            &base,
            opts.stream_id,
            opts.segment_length,
        )?)
    } else {
        None
    };
    // JVM + driver + archive startup.
    std::thread::sleep(std::time::Duration::from_secs(3));
    let endpoints = match &server {
        // A locally launched archive replays to its own configured endpoint;
        // subscribing anywhere else receives nothing. `--replay-channel`
        // overrides it only when the caller names one explicitly.
        Some(s) => {
            let mut e = s.endpoints.clone();
            if let Some(channel) = &opts.replay_channel {
                e.replay = channel.clone();
            }
            e
        }
        None => ArchiveEndpoints {
            control: std::env::var("ERGO_ARCHIVE_CONTROL")
                .unwrap_or_else(|_| "aeron:udp?endpoint=localhost:8010".into()),
            control_response: std::env::var("ERGO_ARCHIVE_CONTROL_RESPONSE")
                .unwrap_or_else(|_| "aeron:udp?endpoint=localhost:8011".into()),
            recording_events: std::env::var("ERGO_ARCHIVE_EVENTS")
                .unwrap_or_else(|_| "aeron:udp?endpoint=localhost:8013".into()),
            // `--replay-channel` first, then the environment: the deployed
            // manifest has to name this pod's own address, and an env var can
            // interpolate the downward-API pod IP where a static arg cannot.
            // Without the fallback the default `localhost:8021` resolves inside
            // the *recorder's* pod, so the Archive reports "no connection
            // established for replayChannel" and the replay delivers nothing.
            replay: opts
                .replay_channel
                .clone()
                .or_else(|| std::env::var("ERGO_REPLAY_CHANNEL").ok())
                .unwrap_or_else(|| "aeron:udp?endpoint=localhost:8021".into()),
            recorded_channel: "aeron:ipc".into(),
            recorded_stream_id: opts.stream_id,
        },
    };
    let aeron_dir = match &server {
        Some(s) => s.aeron_dir.clone(),
        None => std::path::PathBuf::from(&opts.aeron_dir),
    };
    // ── one replay source per Archive (PLAN §9) ──────────────────────────
    //
    // Every recorder pod owns its own Archive behind its own control endpoint,
    // so the sources are a *list*. With no `--archive-source` this is the one
    // env-configured Archive the lab and CI paths use, unchanged.
    let specs: Vec<(String, ArchiveEndpoints)> = if opts.archive_sources.is_empty() {
        vec![("aeron".to_string(), endpoints)]
    } else {
        let mut out = Vec::new();
        for (i, name) in opts.archive_sources.iter().enumerate() {
            out.push((name.clone(), archive_endpoints_for(name, i)?));
        }
        out
    };
    // The one-shot lab path reports a missing recording; the long-running pod
    // path waits, because a producer that has not started yet is not an error.
    let one_shot = opts.publish_export.is_some();
    let replay_stream_id = opts.stream_id + 100;
    let budget = ReplayBudget {
        bytes: ingester.config.replay_budget_bytes,
    };

    let mut clients = Vec::with_capacity(specs.len());
    for (i, (_, ep)) in specs.iter().enumerate() {
        let client = ArchiveClient::connect(ep, &aeron_dir)?;
        if i == 0
            && let Some(path) = &opts.publish_export
        {
            let (sent, subscription_id) =
                publish_export_into_aeron(ep, &aeron_dir, &client.archive, path, opts.stream_id)?;
            client
                .archive
                .stop_recording_subscription(subscription_id)?;
            eprintln!("ingester: published {sent} frames into the archive recording");
        }
        clients.push(client);
    }

    // `attach_archive` takes the client — post-checkpoint pruning and the
    // metric sampler both use it — and later passes borrow it back through
    // `archive_client(idx)`. `ArchiveReplaySource` owns its own Aeron handles
    // and never holds that borrow, so re-resolving between passes is sound.
    let mut sources = Vec::new();
    for ((name, endpoints), client) in specs.into_iter().zip(clients) {
        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        let info = loop {
            if let Some(found) = client.find_recording(&endpoints)
                && (!one_shot || !found.is_active())
            {
                break Some(found);
            }
            if one_shot && std::time::Instant::now() >= deadline {
                break None;
            }
            std::thread::sleep(Duration::from_millis(200));
        };
        let info = info.ok_or_else(|| format!("no stopped recording found on {name}"))?;
        eprintln!(
            "ingester: [{name}] recording {} positions {}..{}",
            info.recording_id, info.start_position, info.stop_position
        );
        ingester.add_source(&format!("aeron:{name}"), opts.run_id)?;
        let idx = ingester.source_count() - 1;
        ingester.attach_archive(client, idx, info);
        sources.push(ReplayState {
            idx,
            name,
            endpoints,
            info,
            drained_stopped: None,
        });
    }

    // One pass drains each source's current recording, then:
    //   * without `--follow`, the run is over. That is what the lab and CI
    //     paths (`--publish-export`, the acceptance scripts) depend on, and it
    //     must stay one-shot so a script can assert on the result.
    //   * with `--follow` — the recorder pods' mode — the ingester instead
    //     waits for more. A process that exits after a single drained
    //     recording is not an error to the StatefulSet, which simply restarts
    //     it forever; that is the CrashLoopBackOff this loop removes, and it
    //     is PLAN §9's long-running ingester.
    loop {
        let mut did_work = false;
        for rs in &mut sources {
            if drain_source(ingester, rs, &aeron_dir, replay_stream_id, &budget)? {
                did_work = true;
            }
        }
        if !opts.follow {
            return Ok(());
        }
        if !did_work {
            std::thread::sleep(Duration::from_millis(500));
        }
    }
}

/// Endpoints for `--archive-source <name>`, resolved through that recorder's
/// headless Service.
///
/// Derived rather than hand-written per recorder, so adding a venue means
/// adding its name. The response and replay channels must name *this* pod on
/// per-index ports, which only the downward API can supply: `localhost` would
/// resolve inside the recorder's own pod, and the Archive then reports "no
/// connection established for replayChannel" while delivering nothing.
#[cfg(feature = "archive")]
fn archive_endpoints_for(
    name: &str,
    index: usize,
) -> Result<ergo_clickhouse_persist::ingest::archive::ArchiveEndpoints, Box<dyn std::error::Error>>
{
    let ns = std::env::var("ERGO_ARCHIVE_NAMESPACE").unwrap_or_else(|_| "clickhouse".into());
    let ip = std::env::var("POD_IP")
        .map_err(|_| "POD_IP must be set to read more than one recorder's Archive")?;
    let host = format!("{name}-archive.{ns}.svc.cluster.local");
    Ok(ergo_clickhouse_persist::ingest::archive::ArchiveEndpoints {
        control: format!("aeron:udp?endpoint={host}:4001"),
        control_response: format!("aeron:udp?endpoint={ip}:{}", 8011 + index),
        recording_events: format!("aeron:udp?endpoint={host}:4003"),
        replay: format!("aeron:udp?endpoint={ip}:{}", 8021 + index),
        recorded_channel: "aeron:ipc".into(),
        recorded_stream_id: 42,
    })
}

/// Per-source state for the replay loop.
#[cfg(feature = "archive")]
struct ReplayState {
    /// Index into the ingester's sources.
    idx: usize,
    name: String,
    endpoints: ergo_clickhouse_persist::ingest::archive::ArchiveEndpoints,
    info: ergo_clickhouse_persist::ingest::archive::RecordingInfo,
    /// A stopped recording that has been fully drained. Re-reading it would
    /// only repeat the same bytes, so it is skipped until the producer
    /// publishes a different one.
    drained_stopped: Option<i64>,
}

/// Drain one source's current recording. Returns whether anything was read.
///
/// Re-resolves first: an active recording grows, and data written after a
/// replay started is not delivered by that replay. A recorder restart
/// publishes a *different* recording entirely.
#[cfg(feature = "archive")]
fn drain_source(
    ingester: &mut Ingester,
    rs: &mut ReplayState,
    aeron_dir: &std::path::Path,
    replay_stream_id: i32,
    budget: &ReplayBudget,
) -> Result<bool, Box<dyn std::error::Error>> {
    use ergo_clickhouse_persist::ingest::archive::ArchiveReplaySource;

    let current = ingester
        .archive_client(rs.idx)
        .and_then(|c| c.find_recording(&rs.endpoints));
    if let Some(next) = current {
        if !next.is_active() && rs.drained_stopped == Some(next.recording_id) {
            return Ok(false);
        }
        if next.recording_id != rs.info.recording_id || next.is_active() {
            rs.info = next;
            ingester.set_recording(rs.idx, rs.info);
        }
    }

    // Per *recording*, not per source: a new recording resumes from its own
    // checkpoint (normally 0), not the previous recording's offset.
    let resume = ingester.retarget_recording(rs.idx, rs.info.recording_id);
    let start = resume.max(rs.info.start_position);
    // An active recording is bounded at wherever it has been written, so a
    // pass whose checkpoint is already at that position has nothing to read —
    // and a zero-length bounded replay is rejected by the Archive. Report
    // "nothing read" and let the caller re-resolve; this is the live-recording
    // equivalent of `drained_stopped`, which only a stopped recording can set.
    if rs.info.is_active() {
        let reached = ingester
            .archive_client(rs.idx)
            .ok_or("archive client not attached")?
            .recording_position(rs.info.recording_id)
            .map_err(|e| format!("recording {} position: {e}", rs.info.recording_id))?;
        if reached <= start {
            return Ok(false);
        }
    }
    eprintln!(
        "ingester: [{}] replaying recording {} from {start} (checkpoint {resume}, archive start {})",
        rs.name, rs.info.recording_id, rs.info.start_position
    );
    let mut source = ArchiveReplaySource::subscribe(
        aeron_dir,
        &rs.endpoints.replay,
        replay_stream_id,
        ingester
            .archive_client(rs.idx)
            .ok_or("archive client not attached")?,
        &rs.info,
        start,
    )?;
    // Sample where the replay *began*, which is `start` — not
    // `source.position()`. The latter reports 0 before anything has drained,
    // which under `--follow` fired every pass and alternated the metric
    // 0, 5634, 0, 5634, breaking the cursor invariant notebook 06 asserts.
    // `start` is both accurate and monotone, and it is what evidences
    // advancement: without it a single-pass run records only its end
    // position, and `verify-aeron`'s "replay position never advanced" check
    // has nothing to compare.
    ingester.sample_aeron_metrics(rs.idx, start);

    // A bounded replay ends when the position reaches a *recorded* stop; a
    // stalled replay is broken out of after 20 empty polls. An active
    // recording has a negative stop position, so it is never "caught up" —
    // the pass idles out and re-resolves, which is how data written after the
    // replay started is seen.
    let mut idle = 0u32;
    let mut read_any = false;
    loop {
        let mut got = false;
        while let Some(batch) = source.next_batch(budget)? {
            ingester.process_batch(rs.idx, &batch)?;
            ingester.flush()?;
            got = true;
            read_any = true;
        }
        ingester.maybe_lifecycle_pass();
        ingester.sample_aeron_metrics(rs.idx, source.position());
        let caught_up = !rs.info.is_active() && source.position() >= rs.info.stop_position;
        if caught_up || idle >= 20 {
            break;
        }
        idle = if got { 0 } else { idle + 1 };
        std::thread::sleep(Duration::from_millis(250));
    }
    let n = ingester.flush()?;
    eprintln!(
        "ingester: [{}] pass complete; {n} rows in the final flush, position {}",
        rs.name,
        source.position()
    );
    if !rs.info.is_active() {
        rs.drained_stopped = Some(rs.info.recording_id);
    }
    Ok(read_any)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut config = IngesterConfig::default();
    let mut export_path: Option<String> = None;
    #[cfg(feature = "archive")]
    let mut aeron: Option<AeronOptions> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--clickhouse-url" => {
                i += 1;
                config.clickhouse_url = args
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| config.clickhouse_url.clone());
            }
            "--database" => {
                i += 1;
                config.database = args
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| config.database.clone());
            }
            "--user" => {
                i += 1;
                config.user = args.get(i).cloned().unwrap_or_else(|| config.user.clone());
            }
            "--password" => {
                i += 1;
                config.password = args.get(i).cloned().unwrap_or_default();
            }
            "--catalog" => {
                i += 1;
                config.catalog_path = args
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| config.catalog_path.clone());
            }
            "--checkpoints" => {
                i += 1;
                config.checkpoint_path = args
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| config.checkpoint_path.clone());
            }
            "--from-export" => {
                i += 1;
                export_path = args.get(i).cloned();
            }
            "--metrics-addr" => {
                i += 1;
                config.metrics_addr = args
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| config.metrics_addr.clone());
            }
            "--mode" => {
                i += 1;
                let mode = args.get(i).cloned().unwrap_or_default();
                match mode.as_str() {
                    "export" => {}
                    #[cfg(feature = "archive")]
                    "aeron" => {
                        aeron.get_or_insert_with(AeronOptions::default);
                    }
                    #[cfg(not(feature = "archive"))]
                    "aeron" => {
                        return Err("built without the `archive` feature".into());
                    }
                    other => {
                        eprintln!("unknown mode: {other}");
                        return Err("bad arguments".into());
                    }
                }
            }
            #[cfg(feature = "archive")]
            other
                if other.starts_with("--archive-")
                    || other.starts_with("--aeron-")
                    || other.starts_with("--replay-")
                    || other == "--stream-id"
                    || other == "--segment-length"
                    || other == "--run-id"
                    || other == "--publish-export"
                    || other == "--follow" =>
            {
                let flag = other.to_string();
                let value = if flag == "--follow" {
                    None
                } else {
                    i += 1;
                    args.get(i).cloned()
                };
                aeron.get_or_insert_with(AeronOptions::default);
                apply_aeron_arg(
                    aeron.as_mut().expect("just inserted"),
                    &flag,
                    value.as_deref(),
                )?;
            }
            "--prune" => {
                config.prune_enabled = true;
            }
            "--batch-rows" => {
                i += 1;
                config.batch_rows = args
                    .get(i)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(config.batch_rows);
            }
            "--batch-bytes" => {
                i += 1;
                config.batch_bytes = args
                    .get(i)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(config.batch_bytes);
            }
            "--help" | "-h" => {
                println!(
                    "ingester — replay sources into ClickHouse\n\nOPTIONS:\n  --clickhouse-url <url>\n  --database <db>\n  --user <user>\n  --password <pw>\n  --catalog <path>\n  --checkpoints <path>\n  --from-export <file>\n  --mode export|aeron\n  --archive-jar <path>       Aeron jar (launches the driver+archive)\n  --archive-base <dir>       Working dir for the launched driver\n  --aeron-dir <dir>          Use an existing driver dir and a REMOTE archive\n  --stream-id <i32>          Recorded stream id\n  --segment-length <bytes>   Archive segment file length\n  --replay-channel <uri>     Replay subscription channel base\n  --run-id <u64>             Producer run id to attribute rows to\n  --publish-export <file>    Publish an export file into Aeron before replaying\n  --follow                   Keep replaying as the recording grows\n  --prune                    Prune archive segments behind the checkpoint"
                );
                return Ok(());
            }
            other => {
                eprintln!("unknown argument: {other}");
                return Err("bad arguments".into());
            }
        }
        i += 1;
    }

    let mut ingester = Ingester::open(config)?;
    eprintln!(
        "ingester ready: catalog={} checkpoints={} clickhouse={}",
        ingester.config.catalog_path,
        ingester.config.checkpoint_path,
        ingester.config.clickhouse_url
    );
    {
        let addr = ingester.config.metrics_addr.clone();
        let histogram = Arc::clone(&ingester.latency);
        std::thread::spawn(move || {
            if let Err(e) = serve_batch_latency_metrics(&addr, histogram) {
                eprintln!("ingester: batch-ack metrics server on {addr} failed: {e:?}");
            }
        });
    }
    if let Some(path) = export_path {
        ingest_export(&mut ingester, &path)?;
    }
    #[cfg(feature = "archive")]
    let had_aeron = aeron.is_some();
    #[cfg(feature = "archive")]
    if let Some(opts) = aeron {
        ingest_aeron(&mut ingester, &opts)?;
    }
    // Pruning removes archive segments behind the checkpoint, so it needs a
    // recording; without one the flag silently does nothing.
    #[cfg(feature = "archive")]
    if ingester.config.prune_enabled && !had_aeron {
        eprintln!(
            "ingester: --prune has no effect in this mode (pruning needs an \
             archive recording; use --mode aeron)"
        );
    }
    Ok(())
}

fn ingest_export(ingester: &mut Ingester, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    let mut frames: Vec<&[u8]> = Vec::new();
    let mut off = 0usize;
    while off + 4 <= bytes.len() {
        let n = u32::from_le_bytes(bytes[off..off + 4].try_into()?) as usize;
        off += 4;
        if off + n > bytes.len() {
            return Err("truncated export frame".into());
        }
        frames.push(&bytes[off..off + n]);
        off += n;
    }
    // The ring must hold every frame at once (the source drains only after all
    // offers). Size it to this export — one slot per frame, each exactly as
    // large as the largest frame. The previous fixed `(8192, 1 MiB)` zeroed
    // 8 GiB up front regardless of the file's actual size.
    let slot_bytes = frames.iter().map(|f| f.len()).max().unwrap_or(1).max(1);
    let mut ring =
        ergo_clickhouse_persist::recorder::MemoryPublication::new(frames.len().max(1), slot_bytes);
    for frame in &frames {
        if !ring.offer(frame) {
            return Err("export frame larger than ingest slot".into());
        }
    }
    ingester.add_source("fixture-export", 1)?;
    let mut source = ergo_clickhouse_persist::ingest::MemorySource::new(ring);
    let budget = ReplayBudget {
        bytes: 8 * 1024 * 1024,
    };
    while let Some(batch) = source.next_batch(&budget)? {
        ingester.process_batch(0, &batch)?;
    }
    let n = ingester.flush()?;
    eprintln!("ingester: flushed {n} rows from {path}");
    Ok(())
}

//! Durable layout-to-storage bindings (SQLite, WAL).
//!
//! Each installed table schema and each layout→storage binding is
//! persisted before data is accepted for it; replay reuses the binding,
//! including the list of incompatible fields it omits, instead of
//! recomputing a different interpretation after a restart.

use crate::ingest::lifecycle::LifecycleState;
use crate::protocol::PolicyDeclaration;
use rusqlite::Connection;

/// One temporary table's lifecycle, durable across restarts.
///
/// PLAN §5 requires the `Active → DropPending → Dropped` progression to be
/// persisted, and the removal of the public view and the backing table to be
/// journalled "so a crash between DDL statements can resume safely". The two
/// removals are separate statements, so the record tracks them separately
/// rather than storing one "dropped" flag that a crash could leave lying.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleRecord {
    /// Producer run id owning the table.
    pub run_id: u64,
    /// Session-local layout id.
    pub layout_id: u16,
    /// Logical table name.
    pub table: String,
    /// Generation, incremented when a dropped table is created again.
    pub generation: u32,
    /// Current lifecycle state.
    pub state: LifecycleState,
    /// Latest row expiry seen in this generation.
    pub latest_expiry_ns: u64,
    /// When this generation last accepted input (drives the idle window).
    pub last_input_ns: u64,
    /// Whether the public view has been removed.
    pub view_dropped: bool,
    /// Whether the backing table has been removed.
    pub backing_dropped: bool,
}

/// Which removal still has to happen for a drop in progress.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropStep {
    /// Remove the public view.
    View,
    /// Remove the backing table.
    Backing,
}

impl LifecycleRecord {
    /// The next removal step, or `None` when both have run.
    ///
    /// Resuming from here is what makes a crash mid-drop safe: the view goes
    /// first because a query hitting the view while the backing table is gone
    /// fails, whereas a surviving backing table behind a dropped view is
    /// merely invisible. Both steps are `IF EXISTS`, so re-running a completed
    /// one is harmless.
    #[must_use]
    pub const fn next_drop_step(&self) -> Option<DropStep> {
        if !self.view_dropped {
            Some(DropStep::View)
        } else if !self.backing_dropped {
            Some(DropStep::Backing)
        } else {
            None
        }
    }
}

/// Durable catalog for the ingester.
pub struct Catalog {
    conn: Connection,
}

/// One frozen binding of a layout to storage.
#[derive(Clone, Debug)]
pub struct Binding {
    /// Producer run id owning the layout.
    pub run_id: u64,
    /// Session-local layout id.
    pub layout_id: u16,
    /// Logical table.
    pub table: String,
    /// Backing table name (namespaced separately from views).
    pub backing: String,
    /// Public view name.
    pub view: String,
    /// Storage column names in order.
    pub columns: Vec<String>,
    /// Columns omitted due to incompatible storage types.
    pub omitted: Vec<String>,
    /// Projection revision the binding was created under.
    pub projection_revision: u32,
    /// Temporary policy for the table.
    pub temporary: bool,
    /// Effective per-row retention in nanoseconds, frozen at bind time.
    ///
    /// PLAN §5 requires a row's expiry to be computed from its capture time
    /// and *that* policy, so a replay must not re-derive it from whatever
    /// config happens to be current. Freezing the value here means the same
    /// event always earns the same expiry. `0` for a permanent table.
    pub row_ttl_ns: u64,
    /// Effective idle-table window in nanoseconds, frozen at bind time.
    ///
    /// The other half of a temporary table's policy: how long it must go
    /// without input before cleanup may consider removing it. Frozen for the
    /// same reason as `row_ttl_ns` — a later config revision must not change
    /// when an already-bound table becomes eligible. `0` for a permanent table.
    pub idle_ttl_ns: u64,
}

impl Catalog {
    /// Open (creating schema if needed) with WAL + FULL sync.
    pub fn open(path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS bindings (
                run_id INTEGER NOT NULL,
                layout_id INTEGER NOT NULL,
                table_name TEXT NOT NULL,
                backing TEXT NOT NULL,
                view TEXT NOT NULL,
                columns TEXT NOT NULL,
                omitted TEXT NOT NULL,
                projection_revision INTEGER NOT NULL,
                temporary INTEGER NOT NULL,
                row_ttl_ns INTEGER NOT NULL DEFAULT 0,
                idle_ttl_ns INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (run_id, layout_id)
            );
            CREATE TABLE IF NOT EXISTS policies (
                run_id INTEGER NOT NULL,
                policy_id INTEGER NOT NULL,
                temporary INTEGER NOT NULL,
                row_ttl_ns INTEGER NOT NULL,
                idle_ttl_ns INTEGER NOT NULL,
                PRIMARY KEY (run_id, policy_id)
            );
            CREATE TABLE IF NOT EXISTS sessions (
                run_id INTEGER PRIMARY KEY,
                process TEXT NOT NULL,
                instance TEXT NOT NULL,
                build TEXT NOT NULL,
                started_at_ns INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS checkpoints (
                source TEXT PRIMARY KEY,
                position INTEGER NOT NULL,
                updated_at_ns INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS lifecycle (
                run_id INTEGER NOT NULL,
                layout_id INTEGER NOT NULL,
                table_name TEXT NOT NULL,
                generation INTEGER NOT NULL,
                state TEXT NOT NULL,
                latest_expiry_ns INTEGER NOT NULL,
                last_input_ns INTEGER NOT NULL,
                view_dropped INTEGER NOT NULL,
                backing_dropped INTEGER NOT NULL,
                PRIMARY KEY (run_id, layout_id)
            );",
        )?;
        // `CREATE TABLE IF NOT EXISTS` silently does nothing when the table is
        // already there, so a catalog on a PVC written by an earlier build
        // would be left without `row_ttl_ns` and every binding write would
        // fail. SQLite has no `ADD COLUMN IF NOT EXISTS`; the duplicate-column
        // error is the idempotency signal, so it is deliberately discarded.
        let _ = conn.execute(
            "ALTER TABLE bindings ADD COLUMN row_ttl_ns INTEGER NOT NULL DEFAULT 0",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE bindings ADD COLUMN idle_ttl_ns INTEGER NOT NULL DEFAULT 0",
            [],
        );
        Ok(Self { conn })
    }

    /// Persist the session start declaration.
    pub fn put_session(
        &self,
        run_id: u64,
        process: &str,
        instance: &str,
        build: &str,
        started_at_ns: u64,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT OR REPLACE INTO sessions VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                run_id as i64,
                process,
                instance,
                build,
                started_at_ns as i64
            ],
        )?;
        Ok(())
    }

    /// Persist a policy declaration.
    pub fn put_policy(&self, run_id: u64, decl: &PolicyDeclaration) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT OR REPLACE INTO policies VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                run_id as i64,
                i64::from(decl.policy_id),
                i64::from(matches!(decl.policy, crate::protocol::Policy::Temporary)),
                decl.row_ttl_ns as i64,
                decl.idle_ttl_ns as i64,
            ],
        )?;
        Ok(())
    }

    /// Resolve a policy declaration.
    #[must_use]
    pub fn policy(&self, run_id: u64, policy_id: u16) -> Option<PolicyDeclaration> {
        self.conn
            .query_row(
                "SELECT temporary, row_ttl_ns, idle_ttl_ns FROM policies \
                 WHERE run_id = ?1 AND policy_id = ?2",
                rusqlite::params![run_id as i64, i64::from(policy_id)],
                |row| {
                    let temporary: i64 = row.get(0)?;
                    Ok(PolicyDeclaration {
                        policy_id,
                        policy: if temporary == 1 {
                            crate::protocol::Policy::Temporary
                        } else {
                            crate::protocol::Policy::Permanent
                        },
                        row_ttl_ns: row.get::<_, i64>(1)? as u64,
                        idle_ttl_ns: row.get::<_, i64>(2)? as u64,
                    })
                },
            )
            .ok()
    }

    /// Persist a frozen layout binding (transactional with DDL confirmation
    /// by the caller).
    pub fn put_binding(&self, b: &Binding) -> Result<(), rusqlite::Error> {
        // Column names are listed explicitly: positional `VALUES` binds by
        // ordinal, so adding a column would silently shift every field after
        // it instead of failing to compile.
        self.conn.execute(
            "INSERT OR REPLACE INTO bindings \
             (run_id, layout_id, table_name, backing, view, columns, omitted, \
              projection_revision, temporary, row_ttl_ns, idle_ttl_ns) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                b.run_id as i64,
                i64::from(b.layout_id),
                b.table,
                b.backing,
                b.view,
                serde_json_names(&b.columns),
                serde_json_names(&b.omitted),
                i64::from(b.projection_revision),
                i64::from(b.temporary),
                b.row_ttl_ns as i64,
                b.idle_ttl_ns as i64,
            ],
        )?;
        Ok(())
    }

    /// Resolve a binding.
    #[must_use]
    pub fn binding(&self, run_id: u64, layout_id: u16) -> Option<Binding> {
        self.conn
            .query_row(
                "SELECT table_name, backing, view, columns, omitted, \
                        projection_revision, temporary, row_ttl_ns, idle_ttl_ns \
                 FROM bindings WHERE run_id = ?1 AND layout_id = ?2",
                rusqlite::params![run_id as i64, i64::from(layout_id)],
                |row| {
                    Ok(Binding {
                        run_id,
                        layout_id,
                        table: row.get(0)?,
                        backing: row.get(1)?,
                        view: row.get(2)?,
                        columns: parse_names(&row.get::<_, String>(3)?),
                        omitted: parse_names(&row.get::<_, String>(4)?),
                        projection_revision: row.get::<_, i64>(5)? as u32,
                        temporary: row.get::<_, i64>(6)? == 1,
                        row_ttl_ns: row.get::<_, i64>(7)? as u64,
                        idle_ttl_ns: row.get::<_, i64>(8)? as u64,
                    })
                },
            )
            .ok()
    }
}

impl Catalog {
    /// Persist a temporary table's lifecycle state.
    ///
    /// Written on every transition, and on every cleanup step, so a restart
    /// resumes from what actually happened rather than from what was intended.
    pub fn put_lifecycle(&self, r: &LifecycleRecord) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT OR REPLACE INTO lifecycle \
             (run_id, layout_id, table_name, generation, state, latest_expiry_ns, \
              last_input_ns, view_dropped, backing_dropped) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                r.run_id as i64,
                i64::from(r.layout_id),
                r.table,
                i64::from(r.generation),
                r.state.as_str(),
                r.latest_expiry_ns as i64,
                r.last_input_ns as i64,
                i64::from(r.view_dropped),
                i64::from(r.backing_dropped),
            ],
        )?;
        Ok(())
    }

    /// Resolve a temporary table's lifecycle state.
    #[must_use]
    pub fn lifecycle(&self, run_id: u64, layout_id: u16) -> Option<LifecycleRecord> {
        self.conn
            .query_row(
                "SELECT table_name, generation, state, latest_expiry_ns, \
                        last_input_ns, view_dropped, backing_dropped \
                 FROM lifecycle WHERE run_id = ?1 AND layout_id = ?2",
                rusqlite::params![run_id as i64, i64::from(layout_id)],
                |row| {
                    let state: String = row.get(2)?;
                    Ok(LifecycleRecord {
                        run_id,
                        layout_id,
                        table: row.get(0)?,
                        generation: row.get::<_, i64>(1)? as u32,
                        // An unparseable state is corruption, not a default;
                        // `Active` is the safe reading because it keeps the
                        // table rather than dropping storage on a bad value.
                        state: LifecycleState::parse(&state).unwrap_or(LifecycleState::Active),
                        latest_expiry_ns: row.get::<_, i64>(3)? as u64,
                        last_input_ns: row.get::<_, i64>(4)? as u64,
                        view_dropped: row.get::<_, i64>(5)? == 1,
                        backing_dropped: row.get::<_, i64>(6)? == 1,
                    })
                },
            )
            .ok()
    }
}

fn serde_json_names(names: &[String]) -> String {
    let mut out = String::from("[");
    for (i, n) in names.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('"');
        out.push_str(&n.replace('\\', "\\\\").replace('"', "\\\""));
        out.push('"');
    }
    out.push(']');
    out
}

fn parse_names(s: &str) -> Vec<String> {
    let inner = s.trim().trim_start_matches('[').trim_end_matches(']');
    if inner.is_empty() {
        return Vec::new();
    }
    inner
        .split(',')
        .map(|p| {
            p.trim()
                .trim_matches('"')
                .replace("\\\"", "\"")
                .replace("\\\\", "\\")
        })
        .collect()
}

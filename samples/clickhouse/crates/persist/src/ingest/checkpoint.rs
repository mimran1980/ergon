//! Source checkpoints: the greatest contiguous acknowledged prefix per
//! recording, persisted transactionally with catalog/binding dependencies.
//!
//! If later batches complete before an earlier one, the checkpoint does
//! not advance past the earlier gap. A recovered checkpoint lying outside
//! the retained Archive history is an explicit error, never a reset.

use rusqlite::Connection;

/// Contiguous-prefix tracker for one source.
///
/// Positions are byte offsets in the source stream, not batch indices, so a
/// batch is contiguous when it *starts* where the previous committed batch
/// ended. Testing `end == committed + 1` would never hold for a real batch:
/// the first one ends at its own length, not at 1.
#[derive(Debug, Default)]
pub struct ContiguousPrefix {
    /// Highest contiguous position committed.
    pub committed: i64,
    /// Completed but not yet contiguous ranges (out-of-order arrivals).
    pub pending: Vec<(i64, i64)>,
}

impl ContiguousPrefix {
    /// Track a completed batch spanning `start..end`; returns the new
    /// committed prefix when the gap closed.
    pub fn complete(&mut self, start: i64, end: i64) -> Option<i64> {
        if start == self.committed && end > start {
            self.committed = end;
            self.pending.sort_unstable();
            while let Some(&(next_start, next_end)) = self.pending.first() {
                if next_start == self.committed && next_end > next_start {
                    self.committed = next_end;
                    self.pending.remove(0);
                } else {
                    break;
                }
            }
            Some(self.committed)
        } else if start > self.committed {
            self.pending.push((start, end));
            None
        } else {
            None // duplicate/older completion: already covered
        }
    }

    /// Outstanding non-contiguous completions.
    #[must_use]
    pub fn pending(&self) -> &[(i64, i64)] {
        &self.pending
    }
}

/// Durable checkpoint store.
pub struct CheckpointStore {
    conn: Connection,
}

impl CheckpointStore {
    /// Open (creating schema if needed).
    pub fn open(path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS checkpoints (
                source TEXT PRIMARY KEY,
                position INTEGER NOT NULL,
                updated_at_ns INTEGER NOT NULL
            );",
        )?;
        Ok(Self { conn })
    }

    /// Persist a checkpoint (after ClickHouse acknowledged the prefix).
    pub fn put(&self, source: &str, position: i64) -> Result<(), rusqlite::Error> {
        let now = crate::registration::now_ns();
        self.conn.execute(
            "INSERT INTO checkpoints VALUES (?1, ?2, ?3) \
             ON CONFLICT(source) DO UPDATE SET position = ?2, updated_at_ns = ?3 \
             WHERE excluded.position > checkpoints.position",
            rusqlite::params![source, position, now as i64],
        )?;
        Ok(())
    }

    /// Load a checkpoint.
    #[must_use]
    pub fn get(&self, source: &str) -> Option<i64> {
        self.conn
            .query_row(
                "SELECT position FROM checkpoints WHERE source = ?1",
                rusqlite::params![source],
                |row| row.get::<_, i64>(0),
            )
            .ok()
    }
}

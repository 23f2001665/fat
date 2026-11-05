use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};

use crate::database::backup;
use crate::database::{migrations, seed};
use crate::error::{AppError, AppResult};

/// Owns the single SQLite connection used by the application.
///
/// Core logic never creates ad-hoc connections. UI code only talks to services,
/// which in turn use repositories with this connection.
pub struct Database {
    conn: Connection,
    path: PathBuf,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> AppResult<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let mut conn = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_FULL_MUTEX,
        )?;
        configure_connection(&conn)?;
        ensure_foreign_keys(&conn)?;

        migrations::run(&mut conn, &path)?;
        seed::seed(&mut conn)?;

        ensure_foreign_keys(&conn)?;
        backup::verify_integrity(&conn)?;
        Ok(Self { conn, path })
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn connection_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn schema_version(&self) -> AppResult<i64> {
        migrations::schema_version(&self.conn)
    }

    /// Creates a validated backup in the database's `backups` directory.
    pub fn backup(&self, label: &str) -> AppResult<PathBuf> {
        let directory = self
            .path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("backups");
        backup::create_backup(&self.conn, &directory, label)
    }

    pub fn verify_integrity(&self) -> AppResult<()> {
        ensure_foreign_keys(&self.conn)?;
        backup::verify_integrity(&self.conn)
    }

    /// Flushes WAL contents into the main database file. Mainly used before
    /// packaging or copying the database outside the app.
    pub fn checkpoint(&self) -> AppResult<()> {
        self.conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }
}

pub fn configure_connection(conn: &Connection) -> AppResult<()> {
    conn.busy_timeout(Duration::from_secs(10))?;
    // Foreign keys must be enabled before any other statement that could use them.
    conn.pragma_update(None, "foreign_keys", "ON")?;
    ensure_foreign_keys(conn)?;
    // WAL survives crashes better and allows a backup reader. This pragma
    // returns the resulting mode; in-memory databases may report "memory".
    let _: String = conn.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    Ok(())
}

/// Called by every repository entry point. This guarantees that a connection
/// cannot silently operate without foreign-key enforcement.
pub fn ensure_foreign_keys(conn: &Connection) -> AppResult<()> {
    let enabled: i64 = conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
    if enabled != 1 {
        return Err(AppError::Integrity(
            "Foreign-key enforcement is not enabled; refusing database operation".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_enables_foreign_keys_and_creates_schema() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("workout.db");
        let db = Database::open(&path).unwrap();
        let fk: i64 = db
            .connection()
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(fk, 1);
        assert!(path.exists());
        assert_eq!(db.schema_version().unwrap(), 1);
    }

    #[test]
    fn reopening_does_not_duplicate_catalogue() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("workout.db");
        let count = {
            let db = Database::open(&path).unwrap();
            db.connection()
                .query_row("SELECT COUNT(*) FROM exercises", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap()
        };
        let db = Database::open(&path).unwrap();
        let count_again: i64 = db
            .connection()
            .query_row("SELECT COUNT(*) FROM exercises", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, count_again);
    }

    #[test]
    fn foreign_key_violation_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(dir.path().join("workout.db")).unwrap();
        let result = db.connection().execute(
            "INSERT INTO workout_logs
                (date, sequence_number, exercise_id, equipment_id, weight, complete_reps, partial_reps)
             VALUES ('2026-11-05', 1, 99999, 1, 20.0, 5, 0)",
            [],
        );
        assert!(result.is_err());
    }

    #[test]
    fn disabled_foreign_keys_are_detected() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(dir.path().join("workout.db")).unwrap();
        db.connection()
            .pragma_update(None, "foreign_keys", "OFF")
            .unwrap();
        let error = ensure_foreign_keys(db.connection()).unwrap_err();
        assert!(error.to_string().contains("Foreign-key enforcement"));
    }
}

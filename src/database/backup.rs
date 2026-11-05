use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{params, Connection, OpenFlags};

use crate::error::{AppError, AppResult};

/// Creates a consistent, validated snapshot of the live database.
///
/// `VACUUM INTO` writes a compact copy without blocking readers. The copy is
/// re-opened and checked with `PRAGMA integrity_check` before it is considered
/// a usable backup, so callers can rely on it before destructive operations.
pub fn create_backup(conn: &Connection, backups_dir: &Path, label: &str) -> AppResult<PathBuf> {
    std::fs::create_dir_all(backups_dir)?;
    let safe_label: String = label
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect();
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S-%3f");
    let mut path = backups_dir.join(format!("{safe_label}-{timestamp}.db"));
    let mut suffix = 1;
    while path.exists() {
        path = backups_dir.join(format!("{safe_label}-{timestamp}-{suffix}.db"));
        suffix += 1;
    }

    conn.execute("VACUUM INTO ?1", params![path.to_string_lossy()])?;
    if let Err(err) = validate_backup(&path) {
        let _ = std::fs::remove_file(&path);
        return Err(err);
    }
    Ok(path)
}

/// Checks that a database file opens, has no corruption, and has no FK violations.
pub fn validate_backup(path: &Path) -> AppResult<()> {
    if !path.exists() {
        return Err(AppError::Migration(format!(
            "Backup file does not exist: {}",
            path.display()
        )));
    }
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|err| AppError::Migration(format!("Backup cannot be opened: {err}")))?;
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|err| AppError::Migration(format!("Backup integrity check failed: {err}")))?;
    if integrity != "ok" {
        return Err(AppError::Migration(format!(
            "Backup integrity check failed: {integrity}"
        )));
    }
    let mut stmt = conn
        .prepare("PRAGMA foreign_key_check")
        .map_err(|err| AppError::Migration(format!("Backup FK check failed: {err}")))?;
    let mut rows = stmt.query([])?;
    if rows.next()?.is_some() {
        return Err(AppError::Migration(
            "Backup contains foreign-key violations".into(),
        ));
    }
    Ok(())
}

/// Restores a validated backup into the live connection.
///
/// The caller must not hold an open transaction. Used only as a last-resort
/// recovery path after a failed migration or interrupted update.
pub fn restore_backup(conn: &mut Connection, backup_path: &Path) -> AppResult<()> {
    validate_backup(backup_path)?;
    let source = Connection::open_with_flags(backup_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    {
        let backup = rusqlite::backup::Backup::new(&source, conn)?;
        backup.run_to_completion(64, Duration::from_millis(10), None)?;
    }
    ensure_fk_enabled(conn)?;
    Ok(())
}

/// Verifies that foreign-key enforcement is active on this connection.
pub fn ensure_fk_enabled(conn: &Connection) -> AppResult<()> {
    let enabled: i64 = conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
    if enabled != 1 {
        return Err(AppError::Migration(
            "Foreign-key enforcement is not enabled on this connection".into(),
        ));
    }
    Ok(())
}

/// Runs the database integrity and foreign-key checks.
pub fn verify_integrity(conn: &Connection) -> AppResult<()> {
    let integrity: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(AppError::Migration(format!(
            "Database integrity check failed: {integrity}"
        )));
    }
    let mut stmt = conn.prepare("PRAGMA foreign_key_check")?;
    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        let table: String = row.get(0).unwrap_or_default();
        return Err(AppError::Migration(format!(
            "Database contains a foreign-key violation in table '{table}'"
        )));
    }
    Ok(())
}

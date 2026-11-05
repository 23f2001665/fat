use std::path::{Path, PathBuf};

use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::database::backup::{self, create_backup, restore_backup, verify_integrity};
use crate::database::connection::ensure_foreign_keys;
use crate::database::schema::{REQUIRED_TABLES, SCHEMA_V1};
use crate::error::{AppError, AppResult};
use crate::utils::date::now_timestamp;

/// A versioned, individually describable database migration.
///
/// The down operation is optional because not every migration can be reversed
/// safely. Destructive migrations must not claim a safe down path.
pub struct Migration {
    pub version: i64,
    pub description: &'static str,
    pub up: fn(&Transaction<'_>) -> rusqlite::Result<()>,
    pub down: Option<fn(&Transaction<'_>) -> rusqlite::Result<()>>,
}

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    description: "Initial workout tracker schema",
    up: |tx| tx.execute_batch(SCHEMA_V1),
    down: None,
}];

pub fn schema_version(conn: &Connection) -> AppResult<i64> {
    Ok(conn.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

fn set_schema_version(conn: &Connection, version: i64) -> AppResult<()> {
    conn.pragma_update(None, "user_version", version)?;
    Ok(())
}

fn backups_dir(db_path: &Path) -> PathBuf {
    db_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("backups")
}

fn table_exists(conn: &Connection, name: &str) -> AppResult<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [name],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn has_user_tables(conn: &Connection) -> AppResult<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master
         WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        [],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

pub fn validate_schema(conn: &Connection, expected_version: i64) -> AppResult<()> {
    for table in REQUIRED_TABLES {
        if !table_exists(conn, table)? {
            return Err(AppError::Migration(format!(
                "Required table '{table}' is missing"
            )));
        }
    }
    let version = schema_version(conn)?;
    if version != expected_version {
        return Err(AppError::Migration(format!(
            "Expected schema version {expected_version}, found {version}"
        )));
    }
    verify_integrity(conn)?;
    Ok(())
}

/// Applies all pending migrations from `migrations`, with a validated backup
/// before touching an existing database.
///
/// Each migration runs in its own `IMMEDIATE` transaction. If anything fails,
/// transactional rollback is attempted first and then the pre-migration backup
/// is restored, preserving the previous working state.
pub(crate) fn migrate_with(
    conn: &mut Connection,
    db_path: &Path,
    migrations: &[Migration],
) -> AppResult<()> {
    ensure_foreign_keys(conn)?;
    if migrations.is_empty() {
        return Ok(());
    }
    let target_version = migrations
        .iter()
        .map(|migration| migration.version)
        .max()
        .unwrap_or(0);
    let current_version = schema_version(conn)?;

    if current_version > target_version {
        return Err(AppError::Migration(format!(
            "Database schema version {current_version} is newer than this application supports \
             (maximum {target_version}). Install a newer application release."
        )));
    }

    if current_version == 0 && has_user_tables(conn)? {
        return Err(AppError::Migration(
            "The database has tables but no schema version; refusing to modify an unversioned database"
                .into(),
        ));
    }

    if current_version == target_version {
        validate_schema(conn, target_version)?;
        return Ok(());
    }

    let backup_path = if current_version > 0 {
        let label = format!("pre-migration-v{current_version}-to-v{target_version}");
        Some(create_backup(conn, &backups_dir(db_path), &label)?)
    } else {
        None
    };

    let apply_result = (|| -> AppResult<()> {
        for migration in migrations
            .iter()
            .filter(|migration| migration.version > current_version)
        {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            (migration.up)(&tx).map_err(|err| {
                AppError::Migration(format!(
                    "Migration {} ({}) failed: {err}",
                    migration.version, migration.description
                ))
            })?;
            tx.execute(
                "INSERT OR REPLACE INTO schema_migrations (version, description, applied_at)
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![migration.version, migration.description, now_timestamp()],
            )?;
            set_schema_version(&tx, migration.version)?;
            tx.commit()?;
            verify_integrity(conn)?;
        }
        validate_schema(conn, target_version)?;
        Ok(())
    })();

    if let Err(error) = apply_result {
        if let Some(path) = backup_path {
            if let Err(restore_error) = restore_backup(conn, &path) {
                return Err(AppError::Migration(format!(
                    "Migration failed ({error}) and automatic restore from {} also failed: {restore_error}",
                    path.display()
                )));
            }
        }
        return Err(error);
    }
    Ok(())
}

pub fn run(conn: &mut Connection, db_path: &Path) -> AppResult<()> {
    migrate_with(conn, db_path, MIGRATIONS)
}

/// Explicitly migrates down to `target_version`.
///
/// The caller is responsible for warning the user that data may be lost. A
/// validated backup is made first. Migrations without a safe down operation
/// block the downgrade rather than silently deleting data.
pub fn downgrade(conn: &mut Connection, db_path: &Path, target_version: i64) -> AppResult<()> {
    ensure_foreign_keys(conn)?;
    if target_version < 0 {
        return Err(AppError::validation(
            "Target schema version cannot be negative",
        ));
    }
    let current = schema_version(conn)?;
    if target_version >= current {
        return Err(AppError::validation(
            "Target schema version must be older than the current version",
        ));
    }
    let backup_path = create_backup(
        conn,
        &backups_dir(db_path),
        &format!("pre-downgrade-v{current}-to-v{target_version}"),
    )?;

    let result = (|| -> AppResult<()> {
        let mut version = current;
        while version > target_version {
            let migration = MIGRATIONS
                .iter()
                .find(|migration| migration.version == version)
                .ok_or_else(|| AppError::Migration(format!("No migration {version} found")))?;
            let down = migration.down.ok_or_else(|| {
                AppError::Migration(format!(
                    "Migration {} ({}) is not safely reversible; refusing downgrade",
                    migration.version, migration.description
                ))
            })?;
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            down(&tx)?;
            let next = version - 1;
            tx.execute(
                "DELETE FROM schema_migrations WHERE version = ?1",
                [version],
            )?;
            set_schema_version(&tx, next)?;
            tx.commit()?;
            version = next;
        }
        verify_integrity(conn)?;
        Ok(())
    })();

    if let Err(error) = result {
        let restore_result = restore_backup(conn, &backup_path);
        return match restore_result {
            Ok(()) => Err(error),
            Err(restore_error) => Err(AppError::Migration(format!(
                "{error}; restoring backup also failed: {restore_error}"
            ))),
        };
    }
    Ok(())
}

/// Recovers a database from a verified backup file.
pub fn restore_validated_backup(conn: &mut Connection, path: &Path) -> AppResult<()> {
    backup::restore_backup(conn, path)?;
    verify_integrity(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::connection::Database;
    use crate::database::SCHEMA_VERSION;

    fn temp_path(name: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        (dir, path)
    }

    #[test]
    fn fresh_database_is_migrated_and_validated() {
        let (_dir, path) = temp_path("fresh.db");
        let db = Database::open(&path).unwrap();
        assert_eq!(schema_version(db.connection()).unwrap(), SCHEMA_VERSION);
        validate_schema(db.connection(), SCHEMA_VERSION).unwrap();
    }

    #[test]
    fn newer_schema_is_rejected_without_modification() {
        let (_dir, path) = temp_path("newer.db");
        {
            let db = Database::open(&path).unwrap();
            db.connection()
                .pragma_update(None, "user_version", 99)
                .unwrap();
        }
        let mut conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        let error = migrate_with(&mut conn, &path, MIGRATIONS).unwrap_err();
        assert!(error.to_string().contains("newer"));
        assert_eq!(schema_version(&conn).unwrap(), 99);
    }

    #[test]
    fn failed_migration_restores_previous_database() {
        let (_dir, path) = temp_path("migration.db");
        {
            let db = Database::open(&path).unwrap();
            db.connection()
                .execute(
                    "INSERT INTO muscle_groups (name, description, is_active)
                     VALUES ('Test Muscle', 'survives', 1)",
                    [],
                )
                .unwrap();
        }

        fn failure(tx: &Transaction<'_>) -> rusqlite::Result<()> {
            tx.execute_batch(
                "CREATE TABLE migration_scratch (id INTEGER);
                 INSERT INTO migration_scratch VALUES (1);
                 SELECT * FROM table_that_does_not_exist;",
            )
        }

        let migrations = &[
            Migration {
                version: 1,
                description: "base",
                up: |tx| tx.execute_batch(SCHEMA_V1),
                down: None,
            },
            Migration {
                version: 2,
                description: "intentional failure",
                up: failure,
                down: None,
            },
        ];

        let mut conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        let error = migrate_with(&mut conn, &path, migrations).unwrap_err();
        assert!(error.to_string().contains("intentional failure"));
        assert_eq!(schema_version(&conn).unwrap(), 1);
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM muscle_groups WHERE name = 'Test Muscle'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
        let scratch: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'migration_scratch'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(scratch, 0);
    }

    #[test]
    fn irreversible_initial_migration_cannot_be_downgraded() {
        let (_dir, path) = temp_path("downgrade.db");
        let mut db = Database::open(&path).unwrap();
        let error = downgrade(db.connection_mut(), &path, 0).unwrap_err();
        assert!(error.to_string().contains("not safely reversible"));
        assert_eq!(schema_version(db.connection()).unwrap(), 1);
    }

    #[test]
    fn backup_can_be_created_and_restored() {
        let (_dir, path) = temp_path("backup.db");
        let mut db = Database::open(&path).unwrap();
        let backups = path.parent().unwrap().join("test-backups");
        let backup = create_backup(db.connection(), &backups, "manual").unwrap();
        backup::validate_backup(&backup).unwrap();

        db.connection()
            .execute(
                "INSERT INTO muscle_groups (name, is_active) VALUES ('Rollback Only', 1)",
                [],
            )
            .unwrap();
        restore_backup(db.connection_mut(), &backup).unwrap();
        let count: i64 = db
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM muscle_groups WHERE name = 'Rollback Only'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }
}

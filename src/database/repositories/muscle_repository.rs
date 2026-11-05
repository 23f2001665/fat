use rusqlite::{params, Connection, OptionalExtension};

use crate::database::connection::ensure_foreign_keys;
use crate::error::{AppError, AppResult};
use crate::models::muscle_group::MuscleGroup;

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MuscleGroup> {
    Ok(MuscleGroup {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        is_active: row.get::<_, i64>(3)? == 1,
    })
}

pub fn list(
    conn: &Connection,
    include_inactive: bool,
    search: Option<&str>,
) -> AppResult<Vec<MuscleGroup>> {
    ensure_foreign_keys(conn)?;
    let query = search.unwrap_or("").trim().to_lowercase();
    let mut stmt = conn.prepare(
        "SELECT id, name, description, is_active
         FROM muscle_groups
         WHERE (?1 = 1 OR is_active = 1)
           AND (?2 = ''
                OR lower(name) LIKE '%' || ?2 || '%'
                OR lower(coalesce(description, '')) LIKE '%' || ?2 || '%')
         ORDER BY is_active DESC, name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map(params![include_inactive as i64, query], map_row)?;
    let mut groups = Vec::new();
    for row in rows {
        groups.push(row?);
    }
    Ok(groups)
}

pub fn get(conn: &Connection, id: i64) -> AppResult<MuscleGroup> {
    ensure_foreign_keys(conn)?;
    conn.query_row(
        "SELECT id, name, description, is_active FROM muscle_groups WHERE id = ?1",
        [id],
        map_row,
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("Muscle group not found"))
}

pub fn name_exists(conn: &Connection, name: &str, excluding_id: Option<i64>) -> AppResult<bool> {
    ensure_foreign_keys(conn)?;
    let exists: i64 = conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM muscle_groups
            WHERE lower(trim(name)) = lower(?1)
              AND (?2 IS NULL OR id <> ?2)
         )",
        params![name.trim(), excluding_id],
        |row| row.get(0),
    )?;
    Ok(exists == 1)
}

pub fn create(conn: &Connection, name: &str, description: Option<&str>) -> AppResult<i64> {
    ensure_foreign_keys(conn)?;
    conn.execute(
        "INSERT INTO muscle_groups (name, description, is_active) VALUES (?1, ?2, 1)",
        params![name, description],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(
    conn: &Connection,
    id: i64,
    name: &str,
    description: Option<&str>,
    is_active: bool,
) -> AppResult<()> {
    ensure_foreign_keys(conn)?;
    let changed = conn.execute(
        "UPDATE muscle_groups SET name = ?2, description = ?3, is_active = ?4 WHERE id = ?1",
        params![id, name, description, is_active as i64],
    )?;
    if changed == 0 {
        return Err(AppError::not_found("Muscle group not found"));
    }
    Ok(())
}

pub fn association_count(conn: &Connection, id: i64) -> AppResult<i64> {
    ensure_foreign_keys(conn)?;
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM exercise_muscles WHERE muscle_group_id = ?1",
        [id],
        |row| row.get(0),
    )?)
}

pub fn can_delete(conn: &Connection, id: i64) -> AppResult<bool> {
    Ok(association_count(conn, id)? == 0)
}

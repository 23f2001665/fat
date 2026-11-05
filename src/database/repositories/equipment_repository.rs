use rusqlite::{params, Connection, OptionalExtension};

use crate::database::connection::ensure_foreign_keys;
use crate::error::{AppError, AppResult};
use crate::models::equipment::{Equipment, NO_EQUIPMENT_ID};

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Equipment> {
    Ok(Equipment {
        id: row.get(0)?,
        name: row.get(1)?,
        category: row.get(2)?,
        is_active: row.get::<_, i64>(3)? == 1,
    })
}

pub fn list(
    conn: &Connection,
    include_inactive: bool,
    search: Option<&str>,
) -> AppResult<Vec<Equipment>> {
    ensure_foreign_keys(conn)?;
    let query = search.unwrap_or("").trim().to_lowercase();
    let mut stmt = conn.prepare(
        "SELECT id, name, category, is_active
         FROM equipment
         WHERE (?1 = 1 OR is_active = 1)
           AND (?2 = ''
                OR lower(name) LIKE '%' || ?2 || '%'
                OR lower(coalesce(category, '')) LIKE '%' || ?2 || '%')
         ORDER BY (id = 1) DESC, is_active DESC, name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map(params![include_inactive as i64, query], map_row)?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

pub fn get(conn: &Connection, id: i64) -> AppResult<Equipment> {
    ensure_foreign_keys(conn)?;
    conn.query_row(
        "SELECT id, name, category, is_active FROM equipment WHERE id = ?1",
        [id],
        map_row,
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("Equipment not found"))
}

pub fn name_exists(conn: &Connection, name: &str, excluding_id: Option<i64>) -> AppResult<bool> {
    ensure_foreign_keys(conn)?;
    let exists: i64 = conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM equipment
            WHERE lower(trim(name)) = lower(?1)
              AND (?2 IS NULL OR id <> ?2)
         )",
        params![name.trim(), excluding_id],
        |row| row.get(0),
    )?;
    Ok(exists == 1)
}

pub fn create(conn: &Connection, name: &str, category: Option<&str>) -> AppResult<i64> {
    ensure_foreign_keys(conn)?;
    conn.execute(
        "INSERT INTO equipment (name, category, is_active) VALUES (?1, ?2, 1)",
        params![name, category],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(
    conn: &Connection,
    id: i64,
    name: &str,
    category: Option<&str>,
    is_active: bool,
) -> AppResult<()> {
    ensure_foreign_keys(conn)?;
    if id == NO_EQUIPMENT_ID && (!is_active || name != "No Equipment") {
        return Err(AppError::validation(
            "The reserved No Equipment record must remain active and keep its name",
        ));
    }
    let changed = conn.execute(
        "UPDATE equipment SET name = ?2, category = ?3, is_active = ?4 WHERE id = ?1",
        params![id, name, category, is_active as i64],
    )?;
    if changed == 0 {
        return Err(AppError::not_found("Equipment not found"));
    }
    Ok(())
}

pub fn set_active(conn: &Connection, id: i64, active: bool) -> AppResult<()> {
    ensure_foreign_keys(conn)?;
    if id == NO_EQUIPMENT_ID && !active {
        return Err(AppError::validation(
            "The reserved No Equipment record cannot be deactivated",
        ));
    }
    let changed = conn.execute(
        "UPDATE equipment SET is_active = ?2 WHERE id = ?1",
        params![id, active as i64],
    )?;
    if changed == 0 {
        return Err(AppError::not_found("Equipment not found"));
    }
    Ok(())
}

pub fn is_referenced_by_workouts(conn: &Connection, id: i64) -> AppResult<bool> {
    ensure_foreign_keys(conn)?;
    let exists: i64 = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM workout_logs WHERE equipment_id = ?1)",
        [id],
        |row| row.get(0),
    )?;
    Ok(exists == 1)
}

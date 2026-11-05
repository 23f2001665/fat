use rusqlite::{params, Connection, OptionalExtension};

use crate::database::connection::ensure_foreign_keys;
use crate::error::{AppError, AppResult};
use crate::models::user::User;
use crate::utils::date::now_timestamp;

pub fn get_local_user(conn: &Connection) -> AppResult<User> {
    ensure_foreign_keys(conn)?;
    conn.query_row(
        "SELECT id, name, height_cm, date_of_birth, created_at, updated_at
         FROM users WHERE id = 1",
        [],
        |row| {
            Ok(User {
                id: row.get(0)?,
                name: row.get(1)?,
                height_cm: row.get(2)?,
                date_of_birth: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("Local user profile is missing"))
}

pub fn update_profile(
    conn: &Connection,
    id: i64,
    name: &str,
    height_cm: Option<f64>,
    date_of_birth: Option<&str>,
) -> AppResult<()> {
    ensure_foreign_keys(conn)?;
    let now = now_timestamp();
    let changed = conn.execute(
        "UPDATE users
         SET name = ?2, height_cm = ?3, date_of_birth = ?4, updated_at = ?5
         WHERE id = ?1",
        params![id, name, height_cm, date_of_birth, now],
    )?;
    if changed == 0 {
        return Err(AppError::not_found("Local user profile is missing"));
    }
    Ok(())
}

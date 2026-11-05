use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

use crate::database::connection::ensure_foreign_keys;
use crate::error::{AppError, AppResult};
use crate::models::exercise::{
    Exercise, ExerciseCategory, ExerciseMuscle, MuscleAssignment, MuscleRole,
};

pub struct ExerciseWrite<'a> {
    pub name: &'a str,
    pub normalized_name: &'a str,
    pub description: Option<&'a str>,
    pub category: ExerciseCategory,
    pub is_active: bool,
    pub muscles: &'a [MuscleAssignment],
}

fn load_muscles(conn: &Connection, exercise_id: i64) -> AppResult<Vec<ExerciseMuscle>> {
    let mut stmt = conn.prepare(
        "SELECT em.muscle_group_id, mg.name, em.role
         FROM exercise_muscles em
         JOIN muscle_groups mg ON mg.id = em.muscle_group_id
         WHERE em.exercise_id = ?1
         ORDER BY CASE em.role WHEN 'Primary' THEN 0 ELSE 1 END, mg.name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([exercise_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    let mut muscles = Vec::new();
    for row in rows {
        let (muscle_group_id, muscle_group_name, role) = row?;
        muscles.push(ExerciseMuscle {
            muscle_group_id,
            muscle_group_name,
            role: MuscleRole::from_db(&role)?,
        });
    }
    Ok(muscles)
}

fn get_raw(conn: &Connection, id: i64) -> AppResult<Option<Exercise>> {
    let raw = conn
        .query_row(
            "SELECT id, name, normalized_name, description, category, is_active
             FROM exercises WHERE id = ?1",
            [id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .optional()?;
    let Some((id, name, normalized_name, description, category, is_active)) = raw else {
        return Ok(None);
    };
    Ok(Some(Exercise {
        id,
        name,
        normalized_name,
        description,
        category: ExerciseCategory::from_db(&category)?,
        is_active: is_active == 1,
        muscles: load_muscles(conn, id)?,
    }))
}

pub fn get(conn: &Connection, id: i64) -> AppResult<Exercise> {
    ensure_foreign_keys(conn)?;
    get_raw(conn, id)?.ok_or_else(|| AppError::not_found("Exercise not found"))
}

pub fn find_by_normalized_name(
    conn: &Connection,
    normalized_name: &str,
) -> AppResult<Option<Exercise>> {
    ensure_foreign_keys(conn)?;
    let id: Option<i64> = conn
        .query_row(
            "SELECT id FROM exercises WHERE normalized_name = ?1",
            [normalized_name],
            |row| row.get(0),
        )
        .optional()?;
    match id {
        Some(id) => get_raw(conn, id),
        None => Ok(None),
    }
}

pub fn list(
    conn: &Connection,
    include_inactive: bool,
    search: Option<&str>,
) -> AppResult<Vec<Exercise>> {
    ensure_foreign_keys(conn)?;
    let query = search.unwrap_or("").trim().to_lowercase();
    let mut stmt = conn.prepare(
        "SELECT id, name, normalized_name, description, category, is_active
         FROM exercises
         WHERE (?1 = 1 OR is_active = 1)
           AND (?2 = ''
                OR lower(name) LIKE '%' || ?2 || '%'
                OR lower(coalesce(description, '')) LIKE '%' || ?2 || '%')
         ORDER BY is_active DESC, name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map(params![include_inactive as i64, query], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, i64>(5)?,
        ))
    })?;
    let mut exercises = Vec::new();
    for row in rows {
        let (id, name, normalized_name, description, category, is_active) = row?;
        exercises.push(Exercise {
            id,
            name,
            normalized_name,
            description,
            category: ExerciseCategory::from_db(&category)?,
            is_active: is_active == 1,
            muscles: load_muscles(conn, id)?,
        });
    }
    Ok(exercises)
}

fn replace_muscles(
    tx: &rusqlite::Transaction<'_>,
    exercise_id: i64,
    muscles: &[MuscleAssignment],
) -> AppResult<()> {
    tx.execute(
        "DELETE FROM exercise_muscles WHERE exercise_id = ?1",
        [exercise_id],
    )?;
    for muscle in muscles {
        tx.execute(
            "INSERT INTO exercise_muscles (exercise_id, muscle_group_id, role)
             VALUES (?1, ?2, ?3)",
            params![exercise_id, muscle.muscle_group_id, muscle.role.as_str()],
        )?;
    }
    Ok(())
}

pub fn create(conn: &mut Connection, input: &ExerciseWrite<'_>) -> AppResult<i64> {
    ensure_foreign_keys(conn)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute(
        "INSERT INTO exercises
            (name, normalized_name, description, category, is_active)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            input.name,
            input.normalized_name,
            input.description,
            input.category.as_str(),
            input.is_active as i64
        ],
    )?;
    let id = tx.last_insert_rowid();
    replace_muscles(&tx, id, input.muscles)?;
    tx.commit()?;
    Ok(id)
}

pub fn update(conn: &mut Connection, id: i64, input: &ExerciseWrite<'_>) -> AppResult<()> {
    ensure_foreign_keys(conn)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let changed = tx.execute(
        "UPDATE exercises
         SET name = ?2, normalized_name = ?3, description = ?4,
             category = ?5, is_active = ?6
         WHERE id = ?1",
        params![
            id,
            input.name,
            input.normalized_name,
            input.description,
            input.category.as_str(),
            input.is_active as i64
        ],
    )?;
    if changed == 0 {
        return Err(AppError::not_found("Exercise not found"));
    }
    replace_muscles(&tx, id, input.muscles)?;
    tx.commit()?;
    Ok(())
}

pub fn set_active(conn: &Connection, id: i64, active: bool) -> AppResult<()> {
    ensure_foreign_keys(conn)?;
    let changed = conn.execute(
        "UPDATE exercises SET is_active = ?2 WHERE id = ?1",
        params![id, active as i64],
    )?;
    if changed == 0 {
        return Err(AppError::not_found("Exercise not found"));
    }
    Ok(())
}

pub fn is_referenced_by_workouts(conn: &Connection, id: i64) -> AppResult<bool> {
    ensure_foreign_keys(conn)?;
    let exists: i64 = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM workout_logs WHERE exercise_id = ?1)",
        [id],
        |row| row.get(0),
    )?;
    Ok(exists == 1)
}

/// Used to prevent deletion through future administration tools. The UI never
/// offers physical deletion, and workout logs use `ON DELETE RESTRICT`.
pub fn can_delete(conn: &Connection, id: i64) -> AppResult<bool> {
    ensure_foreign_keys(conn)?;
    let used: i64 = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM workout_logs WHERE exercise_id = ?1)",
        [id],
        |row| row.get(0),
    )?;
    let linked: i64 = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM exercise_muscles WHERE exercise_id = ?1)",
        [id],
        |row| row.get(0),
    )?;
    Ok(used == 0 && linked == 0)
}

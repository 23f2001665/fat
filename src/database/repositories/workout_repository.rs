use std::collections::HashMap;

use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

use crate::database::connection::ensure_foreign_keys;
use crate::error::{AppError, AppResult};
use crate::models::workout::{NewWorkoutSet, UpdateWorkoutSet, WorkoutDaySummary, WorkoutSet};
use crate::utils::date::{format_date, parse_date};

struct RawSet {
    date: String,
    sequence_number: i64,
    exercise_id: i64,
    exercise_name: String,
    equipment_id: i64,
    equipment_name: String,
    weight: f64,
    complete_reps: i64,
    partial_reps: i64,
    rest_seconds: Option<i64>,
    rir: Option<i64>,
    extra_info: Option<String>,
}

impl TryFrom<RawSet> for WorkoutSet {
    type Error = AppError;

    fn try_from(raw: RawSet) -> Result<Self, Self::Error> {
        Ok(Self {
            date: parse_date(&raw.date)?,
            sequence_number: raw.sequence_number,
            exercise_id: raw.exercise_id,
            exercise_name: raw.exercise_name,
            equipment_id: raw.equipment_id,
            equipment_name: raw.equipment_name,
            weight: raw.weight,
            complete_reps: raw.complete_reps,
            partial_reps: raw.partial_reps,
            rest_seconds: raw.rest_seconds,
            rir: raw.rir,
            extra_info: raw.extra_info,
        })
    }
}

fn map_raw_set(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawSet> {
    Ok(RawSet {
        date: row.get(0)?,
        sequence_number: row.get(1)?,
        exercise_id: row.get(2)?,
        exercise_name: row.get(3)?,
        equipment_id: row.get(4)?,
        equipment_name: row.get(5)?,
        weight: row.get(6)?,
        complete_reps: row.get(7)?,
        partial_reps: row.get(8)?,
        rest_seconds: row.get(9)?,
        rir: row.get(10)?,
        extra_info: row.get(11)?,
    })
}

const SET_SELECT: &str = "SELECT w.date, w.sequence_number, w.exercise_id, e.name,
        w.equipment_id, q.name, w.weight, w.complete_reps, w.partial_reps,
        w.rest_seconds, w.rir, w.extra_info
    FROM workout_logs w
    JOIN exercises e ON e.id = w.exercise_id
    JOIN equipment q ON q.id = w.equipment_id";

pub fn create(conn: &mut Connection, input: &NewWorkoutSet) -> AppResult<i64> {
    ensure_foreign_keys(conn)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let date = format_date(input.date);
    let next_sequence: i64 = tx.query_row(
        "SELECT COALESCE(MAX(sequence_number), 0) + 1
         FROM workout_logs WHERE date = ?1",
        [&date],
        |row| row.get(0),
    )?;
    tx.execute(
        "INSERT INTO workout_logs
            (date, sequence_number, exercise_id, equipment_id, weight,
             complete_reps, partial_reps, rest_seconds, rir, extra_info)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            date,
            next_sequence,
            input.exercise_id,
            input.equipment_id,
            input.weight,
            input.complete_reps,
            input.partial_reps,
            input.rest_seconds,
            input.rir,
            input.extra_info
        ],
    )?;
    tx.commit()?;
    Ok(next_sequence)
}

pub fn list_for_date(conn: &Connection, date: NaiveDate) -> AppResult<Vec<WorkoutSet>> {
    ensure_foreign_keys(conn)?;
    let mut stmt = conn.prepare(&format!(
        "{SET_SELECT} WHERE w.date = ?1 ORDER BY w.date, w.sequence_number"
    ))?;
    let rows = stmt.query_map([format_date(date)], map_raw_set)?;
    let mut sets = Vec::new();
    for row in rows {
        sets.push(WorkoutSet::try_from(row?)?);
    }
    Ok(sets)
}

pub fn get(conn: &Connection, date: NaiveDate, sequence_number: i64) -> AppResult<WorkoutSet> {
    ensure_foreign_keys(conn)?;
    let raw = conn
        .query_row(
            &format!("{SET_SELECT} WHERE w.date = ?1 AND w.sequence_number = ?2"),
            params![format_date(date), sequence_number],
            map_raw_set,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("Workout set not found"))?;
    raw.try_into()
}

pub fn update(
    conn: &Connection,
    date: NaiveDate,
    sequence_number: i64,
    input: &UpdateWorkoutSet,
) -> AppResult<()> {
    ensure_foreign_keys(conn)?;
    let changed = conn.execute(
        "UPDATE workout_logs
         SET exercise_id = ?3, equipment_id = ?4, weight = ?5,
             complete_reps = ?6, partial_reps = ?7,
             rest_seconds = ?8, rir = ?9, extra_info = ?10
         WHERE date = ?1 AND sequence_number = ?2",
        params![
            format_date(date),
            sequence_number,
            input.exercise_id,
            input.equipment_id,
            input.weight,
            input.complete_reps,
            input.partial_reps,
            input.rest_seconds,
            input.rir,
            input.extra_info
        ],
    )?;
    if changed == 0 {
        return Err(AppError::not_found("Workout set not found"));
    }
    Ok(())
}

/// Deletes one set. Deliberately does not renumber later sets: sequence numbers
/// represent stable chronological identity, not a contiguous array index.
pub fn delete(conn: &Connection, date: NaiveDate, sequence_number: i64) -> AppResult<()> {
    ensure_foreign_keys(conn)?;
    let changed = conn.execute(
        "DELETE FROM workout_logs WHERE date = ?1 AND sequence_number = ?2",
        params![format_date(date), sequence_number],
    )?;
    if changed == 0 {
        return Err(AppError::not_found("Workout set not found"));
    }
    Ok(())
}

pub fn list_day_summaries(conn: &Connection) -> AppResult<Vec<WorkoutDaySummary>> {
    ensure_foreign_keys(conn)?;
    let mut stmt = conn.prepare(
        "SELECT date, COUNT(*) AS set_count,
                SUM(weight * (complete_reps + partial_reps / 2.0)) AS total_volume
         FROM workout_logs
         GROUP BY date
         ORDER BY date DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, f64>(2)?,
        ))
    })?;
    let mut summaries = Vec::new();
    for row in rows {
        let (date, set_count, total_volume) = row?;
        summaries.push(WorkoutDaySummary {
            date: parse_date(&date)?,
            set_count,
            total_volume,
            exercise_names: Vec::new(),
        });
    }
    drop(stmt);

    let mut names_stmt = conn.prepare(
        "SELECT w.date, e.name
         FROM workout_logs w
         JOIN exercises e ON e.id = w.exercise_id
         GROUP BY w.date, e.id
         ORDER BY w.date DESC, MIN(w.sequence_number) ASC",
    )?;
    let name_rows = names_stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut names_by_date: HashMap<String, Vec<String>> = HashMap::new();
    for row in name_rows {
        let (date, name) = row?;
        names_by_date.entry(date).or_default().push(name);
    }
    for summary in &mut summaries {
        if let Some(names) = names_by_date.remove(&format_date(summary.date)) {
            summary.exercise_names = names;
        }
    }
    Ok(summaries)
}

pub fn latest_date(conn: &Connection) -> AppResult<Option<NaiveDate>> {
    ensure_foreign_keys(conn)?;
    let date: Option<String> = conn
        .query_row(
            "SELECT date FROM workout_logs ORDER BY date DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    date.map(|value| parse_date(&value)).transpose()
}

pub fn set_count(conn: &Connection, date: NaiveDate) -> AppResult<i64> {
    ensure_foreign_keys(conn)?;
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM workout_logs WHERE date = ?1",
        [format_date(date)],
        |row| row.get(0),
    )?)
}

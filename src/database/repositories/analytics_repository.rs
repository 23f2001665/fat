use chrono::NaiveDate;
use rusqlite::{params, Connection};

use crate::database::connection::ensure_foreign_keys;
use crate::error::AppResult;
use crate::utils::date::{format_date, parse_date, DateRange};

/// SQL applied to workout sets, with optional exercise and muscle filters.
///
/// Parameters used by every query in this module:
/// 1 = start date (nullable), 2 = end date (nullable), 3 = exercise id
/// (nullable), 4 = muscle-group id (nullable).
const SET_FILTER: &str = "
    (?1 IS NULL OR w.date >= ?1)
    AND (?2 IS NULL OR w.date <= ?2)
    AND (?3 IS NULL OR w.exercise_id = ?3)
    AND (?4 IS NULL OR EXISTS (
        SELECT 1 FROM exercise_muscles emf
        WHERE emf.exercise_id = w.exercise_id AND emf.muscle_group_id = ?4
    ))";

#[derive(Debug, Clone, Default)]
pub struct SetAggregate {
    pub total_sets: i64,
    pub total_effective_reps: f64,
    pub total_volume: f64,
    pub max_weight: Option<f64>,
    pub average_weight: Option<f64>,
    pub average_rir: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct DateAggregate {
    pub date: NaiveDate,
    pub max_weight: f64,
    pub volume: f64,
    pub average_rir: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct MuscleAggregate {
    pub muscle_group_id: i64,
    pub name: String,
    pub primary_sets: i64,
    pub secondary_sets: i64,
    pub primary_volume: f64,
    pub secondary_volume: f64,
    pub workout_days: i64,
}

#[derive(Debug, Clone)]
pub struct ExerciseFrequencyRow {
    pub exercise_id: i64,
    pub name: String,
    pub sets: i64,
}

fn optional_dates(range: DateRange) -> (Option<String>, Option<String>) {
    (range.start.map(format_date), range.end.map(format_date))
}

pub fn summarize(
    conn: &Connection,
    range: DateRange,
    exercise_id: Option<i64>,
    muscle_group_id: Option<i64>,
) -> AppResult<SetAggregate> {
    ensure_foreign_keys(conn)?;
    let (start, end) = optional_dates(range);
    let sql = format!(
        "WITH sets AS (
            SELECT w.weight,
                   w.complete_reps + w.partial_reps / 2.0 AS effective_reps,
                   w.weight * (w.complete_reps + w.partial_reps / 2.0) AS volume,
                   w.rir
            FROM workout_logs w
            WHERE {SET_FILTER}
         )
         SELECT COUNT(*),
                COALESCE(SUM(effective_reps), 0),
                COALESCE(SUM(volume), 0),
                MAX(weight),
                AVG(weight),
                AVG(rir)
         FROM sets"
    );
    let aggregate = conn.query_row(
        &sql,
        params![start, end, exercise_id, muscle_group_id],
        |row| {
            Ok(SetAggregate {
                total_sets: row.get(0)?,
                total_effective_reps: row.get(1)?,
                total_volume: row.get(2)?,
                max_weight: row.get(3)?,
                average_weight: row.get(4)?,
                average_rir: row.get(5)?,
            })
        },
    )?;
    Ok(aggregate)
}

pub fn per_date(
    conn: &Connection,
    range: DateRange,
    exercise_id: Option<i64>,
    muscle_group_id: Option<i64>,
) -> AppResult<Vec<DateAggregate>> {
    ensure_foreign_keys(conn)?;
    let (start, end) = optional_dates(range);
    let sql = format!(
        "SELECT w.date,
                MAX(w.weight),
                SUM(w.weight * (w.complete_reps + w.partial_reps / 2.0)),
                AVG(w.rir)
         FROM workout_logs w
         WHERE {SET_FILTER}
         GROUP BY w.date
         ORDER BY w.date ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![start, end, exercise_id, muscle_group_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, f64>(1)?,
            row.get::<_, f64>(2)?,
            row.get::<_, Option<f64>>(3)?,
        ))
    })?;
    let mut result = Vec::new();
    for row in rows {
        let (date, max_weight, volume, average_rir) = row?;
        result.push(DateAggregate {
            date: parse_date(&date)?,
            max_weight,
            volume,
            average_rir,
        });
    }
    Ok(result)
}

/// Primary work is attributed at 100% of a set's volume; secondary involvement
/// at 50%. Explicit primary and secondary values are returned separately so the
/// UI can avoid presenting them as equivalent.
pub fn muscle_groups(
    conn: &Connection,
    range: DateRange,
    exercise_id: Option<i64>,
    muscle_group_id: Option<i64>,
) -> AppResult<Vec<MuscleAggregate>> {
    ensure_foreign_keys(conn)?;
    let (start, end) = optional_dates(range);
    let sql = format!(
        "WITH sets AS (
            SELECT w.date, w.exercise_id,
                   w.weight * (w.complete_reps + w.partial_reps / 2.0) AS volume
            FROM workout_logs w
            WHERE {SET_FILTER}
         )
         SELECT mg.id, mg.name,
                SUM(CASE WHEN em.role = 'Primary' THEN 1 ELSE 0 END) AS primary_sets,
                SUM(CASE WHEN em.role = 'Secondary' THEN 1 ELSE 0 END) AS secondary_sets,
                COALESCE(SUM(CASE WHEN em.role = 'Primary' THEN s.volume ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN em.role = 'Secondary' THEN s.volume ELSE 0 END), 0)
                    * 0.5,
                COUNT(DISTINCT s.date)
         FROM sets s
         JOIN exercise_muscles em ON em.exercise_id = s.exercise_id
         JOIN muscle_groups mg ON mg.id = em.muscle_group_id
         WHERE (?4 IS NULL OR mg.id = ?4)
         GROUP BY mg.id, mg.name
         ORDER BY (primary_sets + secondary_sets) DESC, mg.name COLLATE NOCASE"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![start, end, exercise_id, muscle_group_id], |row| {
        Ok(MuscleAggregate {
            muscle_group_id: row.get(0)?,
            name: row.get(1)?,
            primary_sets: row.get(2)?,
            secondary_sets: row.get(3)?,
            primary_volume: row.get(4)?,
            secondary_volume: row.get(5)?,
            workout_days: row.get(6)?,
        })
    })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn exercise_frequency(
    conn: &Connection,
    range: DateRange,
    exercise_id: Option<i64>,
    muscle_group_id: Option<i64>,
) -> AppResult<Vec<ExerciseFrequencyRow>> {
    ensure_foreign_keys(conn)?;
    let (start, end) = optional_dates(range);
    let sql = format!(
        "SELECT e.id, e.name, COUNT(*)
         FROM workout_logs w
         JOIN exercises e ON e.id = w.exercise_id
         WHERE {SET_FILTER}
         GROUP BY e.id, e.name
         ORDER BY COUNT(*) DESC, e.name COLLATE NOCASE"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![start, end, exercise_id, muscle_group_id], |row| {
        Ok(ExerciseFrequencyRow {
            exercise_id: row.get(0)?,
            name: row.get(1)?,
            sets: row.get(2)?,
        })
    })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn has_data(
    conn: &Connection,
    range: DateRange,
    exercise_id: Option<i64>,
    muscle_group_id: Option<i64>,
) -> AppResult<bool> {
    ensure_foreign_keys(conn)?;
    let (start, end) = optional_dates(range);
    let sql = format!("SELECT EXISTS(SELECT 1 FROM workout_logs w WHERE {SET_FILTER})");
    let exists: i64 = conn.query_row(
        &sql,
        params![start, end, exercise_id, muscle_group_id],
        |row| row.get(0),
    )?;
    Ok(exists == 1)
}

use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension};

use crate::database::connection::ensure_foreign_keys;
use crate::error::{AppError, AppResult};
use crate::models::body_measurement::{BodyMeasurement, NewBodyMeasurement};
use crate::utils::date::{format_date, parse_date};

struct RawMeasurement {
    id: i64,
    user_id: i64,
    measured_at: String,
    weight_kg: Option<f64>,
    height_cm: Option<f64>,
    body_fat_percentage: Option<f64>,
    waist_cm: Option<f64>,
    chest_cm: Option<f64>,
    hip_cm: Option<f64>,
    arm_cm: Option<f64>,
    thigh_cm: Option<f64>,
    notes: Option<String>,
}

impl TryFrom<RawMeasurement> for BodyMeasurement {
    type Error = AppError;

    fn try_from(raw: RawMeasurement) -> Result<Self, Self::Error> {
        Ok(Self {
            id: raw.id,
            user_id: raw.user_id,
            measured_at: parse_date(&raw.measured_at)?,
            weight_kg: raw.weight_kg,
            height_cm: raw.height_cm,
            body_fat_percentage: raw.body_fat_percentage,
            waist_cm: raw.waist_cm,
            chest_cm: raw.chest_cm,
            hip_cm: raw.hip_cm,
            arm_cm: raw.arm_cm,
            thigh_cm: raw.thigh_cm,
            notes: raw.notes,
        })
    }
}

fn map_raw(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawMeasurement> {
    Ok(RawMeasurement {
        id: row.get(0)?,
        user_id: row.get(1)?,
        measured_at: row.get(2)?,
        weight_kg: row.get(3)?,
        height_cm: row.get(4)?,
        body_fat_percentage: row.get(5)?,
        waist_cm: row.get(6)?,
        chest_cm: row.get(7)?,
        hip_cm: row.get(8)?,
        arm_cm: row.get(9)?,
        thigh_cm: row.get(10)?,
        notes: row.get(11)?,
    })
}

const SELECT_COLUMNS: &str = "id, user_id, measured_at, weight_kg, height_cm,
    body_fat_percentage, waist_cm, chest_cm, hip_cm, arm_cm, thigh_cm, notes";

pub fn create(conn: &Connection, user_id: i64, input: &NewBodyMeasurement) -> AppResult<i64> {
    ensure_foreign_keys(conn)?;
    conn.execute(
        "INSERT INTO body_measurements
            (user_id, measured_at, weight_kg, height_cm, body_fat_percentage,
             waist_cm, chest_cm, hip_cm, arm_cm, thigh_cm, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            user_id,
            format_date(input.measured_at),
            input.weight_kg,
            input.height_cm,
            input.body_fat_percentage,
            input.waist_cm,
            input.chest_cm,
            input.hip_cm,
            input.arm_cm,
            input.thigh_cm,
            if input.notes.trim().is_empty() {
                None
            } else {
                Some(input.notes.trim())
            }
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get(conn: &Connection, id: i64) -> AppResult<BodyMeasurement> {
    ensure_foreign_keys(conn)?;
    let raw = conn
        .query_row(
            &format!("SELECT {SELECT_COLUMNS} FROM body_measurements WHERE id = ?1"),
            [id],
            map_raw,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("Measurement not found"))?;
    raw.try_into()
}

pub fn list_all(conn: &Connection, user_id: i64) -> AppResult<Vec<BodyMeasurement>> {
    ensure_foreign_keys(conn)?;
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLUMNS} FROM body_measurements
         WHERE user_id = ?1
         ORDER BY measured_at DESC, id DESC"
    ))?;
    let rows = stmt.query_map([user_id], map_raw)?;
    let mut measurements = Vec::new();
    for row in rows {
        measurements.push(BodyMeasurement::try_from(row?)?);
    }
    Ok(measurements)
}

pub fn list_in_range(
    conn: &Connection,
    user_id: i64,
    start: Option<NaiveDate>,
    end: Option<NaiveDate>,
) -> AppResult<Vec<BodyMeasurement>> {
    ensure_foreign_keys(conn)?;
    let start = start.map(format_date);
    let end = end.map(format_date);
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLUMNS} FROM body_measurements
         WHERE user_id = ?1
           AND (?2 IS NULL OR measured_at >= ?2)
           AND (?3 IS NULL OR measured_at <= ?3)
         ORDER BY measured_at ASC, id ASC"
    ))?;
    let rows = stmt.query_map(params![user_id, start, end], map_raw)?;
    let mut measurements = Vec::new();
    for row in rows {
        measurements.push(BodyMeasurement::try_from(row?)?);
    }
    Ok(measurements)
}

pub fn latest(conn: &Connection, user_id: i64) -> AppResult<Option<BodyMeasurement>> {
    ensure_foreign_keys(conn)?;
    let raw = conn
        .query_row(
            &format!(
                "SELECT {SELECT_COLUMNS} FROM body_measurements
                 WHERE user_id = ?1
                 ORDER BY measured_at DESC, id DESC LIMIT 1"
            ),
            [user_id],
            map_raw,
        )
        .optional()?;
    raw.map(BodyMeasurement::try_from).transpose()
}

pub fn latest_before(
    conn: &Connection,
    user_id: i64,
    measurement: &BodyMeasurement,
) -> AppResult<Option<BodyMeasurement>> {
    ensure_foreign_keys(conn)?;
    let raw = conn
        .query_row(
            &format!(
                "SELECT {SELECT_COLUMNS} FROM body_measurements
                 WHERE user_id = ?1
                   AND (measured_at < ?2 OR (measured_at = ?2 AND id < ?3))
                 ORDER BY measured_at DESC, id DESC LIMIT 1"
            ),
            params![
                user_id,
                format_date(measurement.measured_at),
                measurement.id
            ],
            map_raw,
        )
        .optional()?;
    raw.map(BodyMeasurement::try_from).transpose()
}

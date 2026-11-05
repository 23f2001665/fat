use chrono::NaiveDate;
use rusqlite::Connection;

use crate::database::repositories::{measurement_repository, user_repository};
use crate::error::{AppError, AppResult};
use crate::models::analytics::SeriesPoint;
use crate::models::body_measurement::{
    BodyMeasurement, LatestBodyMetrics, MeasurementMetric, MetricComparison, NewBodyMeasurement,
};
use crate::models::user::User;
use crate::utils::date::{format_date, today_local, DateRange};
use crate::utils::text::{optional_text, required_name, validate_positive, validate_range};

pub fn get_local_user(conn: &Connection) -> AppResult<User> {
    user_repository::get_local_user(conn)
}

pub fn update_profile(
    conn: &Connection,
    name: &str,
    height_cm: Option<f64>,
    date_of_birth: Option<NaiveDate>,
) -> AppResult<()> {
    let name = required_name(name, "Name")?;
    if let Some(height) = height_cm {
        if !height.is_finite() || !(50.0..=300.0).contains(&height) {
            return Err(AppError::validation("Height must be between 50 and 300 cm"));
        }
    }
    if let Some(dob) = date_of_birth {
        if dob > today_local() {
            return Err(AppError::validation(
                "Date of birth cannot be in the future",
            ));
        }
        if dob < NaiveDate::from_ymd_opt(1900, 1, 1).unwrap() {
            return Err(AppError::validation("Date of birth is too far in the past"));
        }
    }
    let dob = date_of_birth.map(format_date);
    let user = user_repository::get_local_user(conn)?;
    user_repository::update_profile(conn, user.id, &name, height_cm, dob.as_deref())
}

pub fn validate_measurement(input: &NewBodyMeasurement) -> AppResult<()> {
    if input.measured_at > today_local() {
        return Err(AppError::validation(
            "A measurement cannot be recorded for a future date",
        ));
    }
    if input.measured_at < NaiveDate::from_ymd_opt(1900, 1, 1).unwrap() {
        return Err(AppError::validation(
            "Measurement date is too far in the past",
        ));
    }
    if !input.has_any_value() {
        return Err(AppError::validation(
            "Enter at least one measurement or a note",
        ));
    }
    if let Some(weight) = input.weight_kg {
        if !weight.is_finite() || weight <= 0.0 || weight > 1_000.0 {
            return Err(AppError::validation(
                "Weight must be greater than 0 and at most 1000 kg",
            ));
        }
    }
    validate_positive(input.height_cm, "Height")?;
    if let Some(height) = input.height_cm {
        if height > 300.0 {
            return Err(AppError::validation("Height must be at most 300 cm"));
        }
    }
    validate_range(input.body_fat_percentage, 0.0, 100.0, "Body-fat percentage")?;
    for (value, label) in [
        (input.waist_cm, "Waist"),
        (input.chest_cm, "Chest"),
        (input.hip_cm, "Hip"),
        (input.arm_cm, "Arm"),
        (input.thigh_cm, "Thigh"),
    ] {
        if let Some(value) = value {
            if !value.is_finite() || !(0.0..=500.0).contains(&value) {
                return Err(AppError::validation(format!(
                    "{label} must be between 0 and 500 cm"
                )));
            }
        }
    }
    optional_text(&input.notes, "Notes")?;
    Ok(())
}

pub fn add_measurement(conn: &Connection, input: &NewBodyMeasurement) -> AppResult<i64> {
    validate_measurement(input)?;
    let user = user_repository::get_local_user(conn)?;
    measurement_repository::create(conn, user.id, input)
}

pub fn measurement_history(conn: &Connection) -> AppResult<Vec<BodyMeasurement>> {
    let user = user_repository::get_local_user(conn)?;
    measurement_repository::list_all(conn, user.id)
}

pub fn latest_measurement(conn: &Connection) -> AppResult<Option<BodyMeasurement>> {
    let user = user_repository::get_local_user(conn)?;
    measurement_repository::latest(conn, user.id)
}

pub fn latest_metrics(conn: &Connection) -> AppResult<LatestBodyMetrics> {
    let history = measurement_history(conn)?;
    let mut metrics = LatestBodyMetrics {
        latest_measurement_date: history.first().map(|measurement| measurement.measured_at),
        ..Default::default()
    };
    for measurement in &history {
        if metrics.weight_kg.is_none() {
            metrics.weight_kg = measurement.weight_kg;
        }
        if metrics.height_cm.is_none() {
            metrics.height_cm = measurement.height_cm;
        }
        if metrics.body_fat_percentage.is_none() {
            metrics.body_fat_percentage = measurement.body_fat_percentage;
        }
        if metrics.waist_cm.is_none() {
            metrics.waist_cm = measurement.waist_cm;
        }
        if metrics.chest_cm.is_none() {
            metrics.chest_cm = measurement.chest_cm;
        }
        if metrics.hip_cm.is_none() {
            metrics.hip_cm = measurement.hip_cm;
        }
        if metrics.arm_cm.is_none() {
            metrics.arm_cm = measurement.arm_cm;
        }
        if metrics.thigh_cm.is_none() {
            metrics.thigh_cm = measurement.thigh_cm;
        }
        if metrics.bmi.is_none() {
            if let Some(bmi) = measurement.bmi() {
                metrics.bmi = Some(bmi);
                metrics.bmi_measured_at = Some(measurement.measured_at);
            }
        }
    }
    Ok(metrics)
}

/// Compares the latest two observations for each metric. Using per-field
/// observations avoids a misleading comparison when, for example, the latest
/// snapshot only contains body weight.
pub fn comparisons(conn: &Connection) -> AppResult<Vec<MetricComparison>> {
    let history = measurement_history(conn)?;
    let mut comparisons: Vec<MetricComparison> = MeasurementMetric::ALL
        .iter()
        .map(|metric| MetricComparison {
            metric: *metric,
            current: None,
            current_date: None,
            previous: None,
            previous_date: None,
        })
        .collect();

    for measurement in &history {
        for comparison in &mut comparisons {
            if let Some(value) = comparison.metric.value(measurement) {
                if comparison.current.is_none() {
                    comparison.current = Some(value);
                    comparison.current_date = Some(measurement.measured_at);
                } else if comparison.previous.is_none() {
                    comparison.previous = Some(value);
                    comparison.previous_date = Some(measurement.measured_at);
                }
            }
        }
    }
    Ok(comparisons)
}

pub fn comparison_for(
    conn: &Connection,
    metric: MeasurementMetric,
) -> AppResult<Option<MetricComparison>> {
    Ok(comparisons(conn)?
        .into_iter()
        .find(|comparison| comparison.metric == metric))
}

/// Returns only actual observations for a metric. No missing dates are filled
/// in or interpolated.
pub fn metric_series(
    conn: &Connection,
    metric: MeasurementMetric,
    range: DateRange,
) -> AppResult<Vec<SeriesPoint>> {
    let user = user_repository::get_local_user(conn)?;
    let measurements =
        measurement_repository::list_in_range(conn, user.id, range.start, range.end)?;
    Ok(measurements
        .iter()
        .filter_map(|measurement| {
            metric.value(measurement).map(|value| SeriesPoint {
                date: measurement.measured_at,
                value,
            })
        })
        .collect())
}

/// Previous record relative to the latest snapshot, retained for detailed
/// side-by-side history views.
pub fn previous_measurement(conn: &Connection) -> AppResult<Option<BodyMeasurement>> {
    let user = user_repository::get_local_user(conn)?;
    match measurement_repository::latest(conn, user.id)? {
        Some(latest) => measurement_repository::latest_before(conn, user.id, &latest),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::utils::calculations::bmi;

    fn setup() -> (tempfile::TempDir, Database) {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(dir.path().join("profile.db")).unwrap();
        (dir, db)
    }

    fn measurement(date: &str, weight: Option<f64>, height: Option<f64>) -> NewBodyMeasurement {
        NewBodyMeasurement {
            measured_at: NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(),
            weight_kg: weight,
            height_cm: height,
            ..Default::default()
        }
    }

    #[test]
    fn bmi_uses_values_from_the_same_snapshot() {
        let (_dir, db) = setup();
        add_measurement(
            db.connection(),
            &measurement("2026-01-01", Some(80.0), None),
        )
        .unwrap();
        add_measurement(
            db.connection(),
            &measurement("2026-02-01", None, Some(180.0)),
        )
        .unwrap();
        let latest = latest_metrics(db.connection()).unwrap();
        assert_eq!(latest.weight_kg, Some(80.0));
        assert_eq!(latest.height_cm, Some(180.0));
        assert_eq!(latest.bmi, None);

        add_measurement(
            db.connection(),
            &measurement("2026-03-01", Some(75.0), Some(180.0)),
        )
        .unwrap();
        let latest = latest_metrics(db.connection()).unwrap();
        assert_eq!(latest.bmi, bmi(Some(75.0), Some(180.0)));
        assert_eq!(
            latest.bmi_measured_at,
            Some(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap())
        );
    }

    #[test]
    fn historical_measurements_are_never_overwritten() {
        let (_dir, db) = setup();
        add_measurement(
            db.connection(),
            &measurement("2026-01-01", Some(80.0), None),
        )
        .unwrap();
        add_measurement(
            db.connection(),
            &measurement("2026-02-01", Some(78.0), None),
        )
        .unwrap();
        update_profile(db.connection(), "Alex", Some(180.0), None).unwrap();
        let history = measurement_history(db.connection()).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].weight_kg, Some(78.0));
        assert_eq!(history[1].weight_kg, Some(80.0));
    }

    #[test]
    fn partial_snapshots_are_allowed() {
        let (_dir, db) = setup();
        let mut partial = measurement("2026-01-01", None, None);
        partial.waist_cm = Some(80.0);
        add_measurement(db.connection(), &partial).unwrap();
        let latest = latest_metrics(db.connection()).unwrap();
        assert_eq!(latest.waist_cm, Some(80.0));
        assert_eq!(latest.weight_kg, None);
    }

    #[test]
    fn empty_measurements_are_rejected() {
        let (_dir, db) = setup();
        let empty = measurement("2026-01-01", None, None);
        assert!(add_measurement(db.connection(), &empty).is_err());
    }

    #[test]
    fn body_fat_percentage_is_bounded() {
        let (_dir, db) = setup();
        let mut input = measurement("2026-01-01", Some(80.0), None);
        input.body_fat_percentage = Some(101.0);
        assert!(add_measurement(db.connection(), &input).is_err());
    }

    #[test]
    fn comparisons_use_latest_and_previous_observations_per_metric() {
        let (_dir, db) = setup();
        add_measurement(
            db.connection(),
            &measurement("2026-01-01", Some(80.0), None),
        )
        .unwrap();
        add_measurement(
            db.connection(),
            &measurement("2026-02-01", Some(79.0), None),
        )
        .unwrap();
        let weight = comparison_for(db.connection(), MeasurementMetric::Weight)
            .unwrap()
            .unwrap();
        assert_eq!(weight.current, Some(79.0));
        assert_eq!(weight.previous, Some(80.0));
        assert_eq!(weight.delta(), Some(-1.0));
    }

    #[test]
    fn chart_series_contains_actual_dates_only() {
        let (_dir, db) = setup();
        add_measurement(
            db.connection(),
            &measurement("2026-01-01", Some(80.0), None),
        )
        .unwrap();
        add_measurement(
            db.connection(),
            &measurement("2026-01-05", Some(79.0), None),
        )
        .unwrap();
        add_measurement(
            db.connection(),
            &measurement("2026-01-09", None, Some(180.0)),
        )
        .unwrap();
        let series = metric_series(
            db.connection(),
            MeasurementMetric::Weight,
            DateRange::all_time(),
        )
        .unwrap();
        assert_eq!(series.len(), 2);
        assert_eq!(series[0].date, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        assert_eq!(series[1].value, 79.0);
    }

    #[test]
    fn profile_edit_does_not_create_a_measurement() {
        let (_dir, db) = setup();
        update_profile(
            db.connection(),
            "Sam",
            Some(175.0),
            Some(NaiveDate::from_ymd_opt(1990, 5, 4).unwrap()),
        )
        .unwrap();
        let user = get_local_user(db.connection()).unwrap();
        assert_eq!(user.name, "Sam");
        assert_eq!(user.height_cm, Some(175.0));
        assert_eq!(measurement_history(db.connection()).unwrap().len(), 0);
    }
}

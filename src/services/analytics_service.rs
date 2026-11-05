use std::collections::BTreeMap;

use chrono::NaiveDate;
use rusqlite::Connection;

use crate::database::repositories::analytics_repository;
use crate::database::repositories::exercise_repository;
use crate::database::repositories::muscle_repository;
use crate::error::AppResult;
use crate::models::analytics::{
    AnalyticsReport, ExerciseAnalytics, MuscleGroupStats, NamedValue, SeriesPoint,
};
use crate::utils::date::{today_local, week_start, Period};

/// Builds a report from actual workout records only.
///
/// - Exercise weight progression uses the heaviest set per workout date.
/// - Volume uses `weight * (complete + partial / 2)`.
/// - Primary muscle volume is attributed at 100%, secondary at 50%; the two
///   components remain separate in the returned data.
/// - Missing dates are never synthesized or interpolated.
pub fn build_report(
    conn: &Connection,
    period: Period,
    exercise_id: Option<i64>,
    muscle_group_id: Option<i64>,
    today: NaiveDate,
) -> AppResult<AnalyticsReport> {
    let range = period.resolve(today)?;
    let exercise_name = match exercise_id {
        Some(id) => Some(exercise_repository::get(conn, id)?.name),
        None => None,
    };
    if let Some(id) = muscle_group_id {
        muscle_repository::get(conn, id)?;
    }

    let summary = analytics_repository::summarize(conn, range, exercise_id, muscle_group_id)?;
    let per_date = analytics_repository::per_date(conn, range, exercise_id, muscle_group_id)?;
    let muscle_rows =
        analytics_repository::muscle_groups(conn, range, exercise_id, muscle_group_id)?;
    let frequency_rows =
        analytics_repository::exercise_frequency(conn, range, exercise_id, muscle_group_id)?;
    let has_data = analytics_repository::has_data(conn, range, exercise_id, muscle_group_id)?;

    let weight_progression = per_date
        .iter()
        .map(|point| SeriesPoint {
            date: point.date,
            value: point.max_weight,
        })
        .collect();
    let volume_progression = per_date
        .iter()
        .map(|point| SeriesPoint {
            date: point.date,
            value: point.volume,
        })
        .collect();
    let rir_trend = per_date
        .iter()
        .filter_map(|point| {
            point.average_rir.map(|value| SeriesPoint {
                date: point.date,
                value,
            })
        })
        .collect();

    let mut weekly: BTreeMap<NaiveDate, f64> = BTreeMap::new();
    for point in &per_date {
        *weekly.entry(week_start(point.date)).or_insert(0.0) += point.volume;
    }
    let weekly_volume = weekly
        .into_iter()
        .map(|(date, value)| SeriesPoint { date, value })
        .collect();

    let muscle_groups: Vec<MuscleGroupStats> = muscle_rows
        .into_iter()
        .map(|row| MuscleGroupStats {
            muscle_group_id: row.muscle_group_id,
            name: row.name,
            primary_sets: row.primary_sets,
            secondary_sets: row.secondary_sets,
            primary_volume: row.primary_volume,
            secondary_volume: row.secondary_volume,
            workout_days: row.workout_days,
        })
        .collect();
    let muscle_distribution = muscle_groups
        .iter()
        .map(|row| NamedValue {
            label: row.name.clone(),
            value: row.total_volume_attributed(),
        })
        .collect();
    let exercise_frequency = frequency_rows
        .into_iter()
        .map(|row| NamedValue {
            label: row.name,
            value: row.sets as f64,
        })
        .collect();

    Ok(AnalyticsReport {
        period,
        range,
        exercise: ExerciseAnalytics {
            exercise_id,
            exercise_name,
            total_sets: summary.total_sets,
            total_effective_reps: summary.total_effective_reps,
            total_volume: summary.total_volume,
            max_weight: summary.max_weight,
            average_weight: summary.average_weight,
            average_rir: summary.average_rir,
            weight_progression,
            volume_progression,
            rir_trend,
        },
        muscle_groups,
        weekly_volume,
        muscle_distribution,
        exercise_frequency,
        has_data,
    })
}

pub fn build_report_for_today(
    conn: &Connection,
    period: Period,
    exercise_id: Option<i64>,
    muscle_group_id: Option<i64>,
) -> AppResult<AnalyticsReport> {
    build_report(conn, period, exercise_id, muscle_group_id, today_local())
}

pub fn total_weekly_volume(conn: &Connection, period: Period) -> AppResult<Vec<SeriesPoint>> {
    Ok(build_report_for_today(conn, period, None, None)?.weekly_volume)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::services::workout_service::NewSetInput;
    use crate::services::{library_service, workout_service};
    use crate::utils::date::DateRange;

    fn setup() -> (tempfile::TempDir, Database, i64, i64, i64) {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path().join("analytics.db")).unwrap();
        let exercises = library_service::list_exercises(db.connection(), true, None).unwrap();
        let squat = exercises
            .iter()
            .find(|exercise| exercise.name == "Barbell Back Squat")
            .unwrap()
            .id;
        let bench = exercises
            .iter()
            .find(|exercise| exercise.name == "Bench Press")
            .unwrap()
            .id;
        let chest = library_service::list_muscle_groups(db.connection(), true, None)
            .unwrap()
            .into_iter()
            .find(|group| group.name == "Chest")
            .unwrap()
            .id;
        let add = |db: &mut Database, date: &str, exercise_id: i64, weight: f64, reps: i64| {
            workout_service::add_set(
                db.connection_mut(),
                &NewSetInput {
                    date: NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(),
                    exercise_id,
                    equipment_id: 2,
                    weight,
                    complete_reps: reps,
                    partial_reps: 0,
                    rest_seconds: Some(120),
                    rir: Some(2),
                    extra_info: String::new(),
                },
            )
            .unwrap();
        };
        add(&mut db, "2026-01-01", squat, 100.0, 5);
        add(&mut db, "2026-01-01", squat, 110.0, 3);
        add(&mut db, "2026-01-02", bench, 80.0, 8);
        add(&mut db, "2026-01-08", squat, 120.0, 2);
        (dir, db, squat, bench, chest)
    }

    fn all_time() -> Period {
        Period::AllTime
    }

    #[test]
    fn exercise_metrics_use_actual_sets() {
        let (_dir, db, squat, _, _) = setup();
        let report = build_report(
            db.connection(),
            all_time(),
            Some(squat),
            None,
            NaiveDate::from_ymd_opt(2026, 11, 5).unwrap(),
        )
        .unwrap();
        assert_eq!(report.exercise.total_sets, 3);
        assert_eq!(
            report.exercise.total_volume,
            100.0 * 5.0 + 110.0 * 3.0 + 120.0 * 2.0
        );
        assert_eq!(report.exercise.max_weight, Some(120.0));
        assert_eq!(report.exercise.weight_progression.len(), 2);
        assert!(report.has_data);
    }

    #[test]
    fn muscle_volume_distinguishes_primary_and_secondary() {
        let (_dir, db, squat, _, _) = setup();
        let report = build_report(
            db.connection(),
            all_time(),
            Some(squat),
            None,
            NaiveDate::from_ymd_opt(2026, 11, 5).unwrap(),
        )
        .unwrap();
        let quads = report
            .muscle_groups
            .iter()
            .find(|group| group.name == "Quadriceps")
            .unwrap();
        let glutes = report
            .muscle_groups
            .iter()
            .find(|group| group.name == "Glutes")
            .unwrap();
        assert_eq!(quads.primary_volume, report.exercise.total_volume);
        assert_eq!(quads.secondary_volume, 0.0);
        assert_eq!(glutes.primary_volume, 0.0);
        assert_eq!(glutes.secondary_volume, report.exercise.total_volume * 0.5);
        assert_eq!(glutes.primary_sets, 0);
        assert_eq!(glutes.secondary_sets, 3);
    }

    #[test]
    fn muscle_filter_applies_to_all_metrics() {
        let (_dir, db, _, bench, chest) = setup();
        let report = build_report(
            db.connection(),
            all_time(),
            None,
            Some(chest),
            NaiveDate::from_ymd_opt(2026, 11, 5).unwrap(),
        )
        .unwrap();
        assert_eq!(report.exercise.total_sets, 1);
        assert_eq!(report.exercise.exercise_id, None);
        assert!(report
            .exercise_frequency
            .iter()
            .all(|row| row.label != "Barbell Back Squat"));
        assert_eq!(report.exercise_frequency[0].value, 1.0);
        let _ = bench;
    }

    #[test]
    fn weekly_volume_uses_workout_weeks_and_no_empty_weeks() {
        let (_dir, db, _, _, _) = setup();
        let report = build_report(
            db.connection(),
            all_time(),
            None,
            None,
            NaiveDate::from_ymd_opt(2026, 11, 5).unwrap(),
        )
        .unwrap();
        assert_eq!(report.weekly_volume.len(), 2);
        assert_eq!(
            report.weekly_volume[0].date,
            NaiveDate::from_ymd_opt(2025, 12, 29).unwrap()
        );
        assert_eq!(
            report.weekly_volume[1].date,
            NaiveDate::from_ymd_opt(2026, 1, 5).unwrap()
        );
    }

    #[test]
    fn empty_period_has_empty_state() {
        let (_dir, db, _, _, _) = setup();
        let period = Period::Custom(
            DateRange::new(
                Some(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()),
                Some(NaiveDate::from_ymd_opt(2026, 3, 31).unwrap()),
            )
            .unwrap(),
        );
        let report = build_report(
            db.connection(),
            period,
            None,
            None,
            NaiveDate::from_ymd_opt(2026, 11, 5).unwrap(),
        )
        .unwrap();
        assert!(!report.has_data);
        assert_eq!(report.exercise.total_sets, 0);
        assert!(report.muscle_groups.is_empty());
        assert!(report.exercise.weight_progression.is_empty());
    }

    #[test]
    fn rir_trend_ignores_missing_rir_values() {
        let (_dir, mut db, squat, _, _) = setup();
        workout_service::add_set(
            db.connection_mut(),
            &NewSetInput {
                date: NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
                exercise_id: squat,
                equipment_id: 2,
                weight: 90.0,
                complete_reps: 5,
                partial_reps: 0,
                rest_seconds: None,
                rir: None,
                extra_info: String::new(),
            },
        )
        .unwrap();
        let report = build_report(
            db.connection(),
            all_time(),
            Some(squat),
            None,
            NaiveDate::from_ymd_opt(2026, 11, 5).unwrap(),
        )
        .unwrap();
        assert!(report
            .exercise
            .rir_trend
            .iter()
            .all(|point| point.value == 2.0));
    }
}

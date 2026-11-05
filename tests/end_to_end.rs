//! End-to-end tests across the database, repositories, and services.
//!
//! These tests exercise the public API the way the UI does, while also probing
//! the SQL constraints directly so a future repository refactor cannot silently
//! weaken the schema.

use chrono::Duration;
use fat::database::{migrations, Database};
use fat::models::body_measurement::NewBodyMeasurement;
use fat::models::equipment::NO_EQUIPMENT_ID;
use fat::models::exercise::{ExerciseCategoryInput, MuscleAssignment, MuscleRole};
use fat::services::workout_service::{EditSetInput, NewSetInput};
use fat::services::{analytics_service, library_service, profile_service, workout_service};
use fat::utils::date::{format_date, today_local, Period};

fn open() -> (tempfile::TempDir, Database) {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("workout.db")).unwrap();
    (dir, db)
}

fn seeded_exercise(db: &Database, name: &str) -> i64 {
    library_service::list_exercises(db.connection(), true, Some(name))
        .unwrap()
        .into_iter()
        .find(|exercise| exercise.name == name)
        .unwrap_or_else(|| panic!("seed exercise '{name}' is missing"))
        .id
}

fn seeded_muscle(db: &Database, name: &str) -> i64 {
    library_service::list_muscle_groups(db.connection(), true, Some(name))
        .unwrap()
        .into_iter()
        .find(|group| group.name == name)
        .unwrap_or_else(|| panic!("seed muscle '{name}' is missing"))
        .id
}

fn set_input(date: chrono::NaiveDate, exercise_id: i64, weight: f64, reps: i64) -> NewSetInput {
    NewSetInput {
        date,
        exercise_id,
        equipment_id: 2,
        weight,
        complete_reps: reps,
        partial_reps: 0,
        rest_seconds: Some(120),
        rir: Some(2),
        extra_info: String::new(),
    }
}

#[test]
fn workout_lifecycle_preserves_sequence_identity_and_survives_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("workout.db");
    let day_one = today_local() - Duration::days(2);
    let day_two = today_local() - Duration::days(1);
    let mut db = Database::open(&path).unwrap();

    let squat = seeded_exercise(&db, "Barbell Back Squat");
    let bench = seeded_exercise(&db, "Bench Press");
    let mut first = set_input(day_one, squat, 100.0, 5);
    first.partial_reps = 2;
    let sequence_one = workout_service::add_set(db.connection_mut(), &first).unwrap();
    let sequence_two =
        workout_service::add_set(db.connection_mut(), &set_input(day_one, bench, 80.0, 8)).unwrap();
    let sequence_three =
        workout_service::add_set(db.connection_mut(), &set_input(day_one, squat, 110.0, 3))
            .unwrap();

    assert_eq!((sequence_one, sequence_two, sequence_three), (1, 2, 3));
    // Sequence numbers restart for a new calendar day.
    let next_day =
        workout_service::add_set(db.connection_mut(), &set_input(day_two, squat, 105.0, 5))
            .unwrap();
    assert_eq!(next_day, 1);

    workout_service::delete_set(db.connection(), day_one, 2).unwrap();
    let day = workout_service::workout_for_date(db.connection(), day_one).unwrap();
    assert_eq!(
        day.sets
            .iter()
            .map(|set| set.sequence_number)
            .collect::<Vec<_>>(),
        vec![1, 3],
        "deleting a middle set must not renumber the remaining sets"
    );
    let effective = day
        .sets
        .iter()
        .find(|set| set.sequence_number == 1)
        .unwrap();
    assert_eq!(effective.effective_reps(), 6.0);
    assert_eq!(effective.volume(), 600.0);
    assert_eq!(day.total_volume(), 600.0 + 330.0);

    // Edits stay on the same sequence number and cannot move a set to a future date.
    workout_service::edit_set(
        db.connection(),
        day_one,
        1,
        &EditSetInput {
            exercise_id: squat,
            equipment_id: 2,
            weight: 102.5,
            complete_reps: 4,
            partial_reps: 2,
            rest_seconds: None,
            rir: Some(1),
            extra_info: "top set".into(),
        },
    )
    .unwrap();
    let edited = workout_service::get_set(db.connection(), day_one, 1).unwrap();
    assert_eq!(edited.weight, 102.5);
    assert_eq!(edited.volume(), 102.5 * 5.0);
    assert_eq!(edited.extra_info.as_deref(), Some("top set"));

    let error = workout_service::add_set(
        db.connection_mut(),
        &set_input(today_local() + Duration::days(1), squat, 100.0, 5),
    )
    .unwrap_err();
    assert!(error.to_string().contains("future"));

    drop(db);
    let db = Database::open(&path).unwrap();
    let day = workout_service::workout_for_date(db.connection(), day_one).unwrap();
    assert_eq!(day.sets.len(), 2);
    assert_eq!(day.sets[0].sequence_number, 1);
    assert_eq!(day.sets[0].weight, 102.5);
    assert_eq!(day.sets[1].sequence_number, 3);
    let history = workout_service::history(db.connection()).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].date, day_two);
    assert_eq!(history[1].exercise_names, vec!["Barbell Back Squat"]);
}

#[test]
fn sql_constraints_reject_invalid_workout_rows_and_protect_references() {
    let (_dir, db) = open();
    let squat = seeded_exercise(&db, "Barbell Back Squat");
    let date = format_date(today_local() - Duration::days(1));
    let conn = db.connection();

    let insert = |sequence: i64,
                  exercise_id: i64,
                  equipment_id: i64,
                  weight: f64,
                  reps: i64,
                  partial: i64,
                  rir: Option<i64>| {
        conn.execute(
            "INSERT INTO workout_logs
                (date, sequence_number, exercise_id, equipment_id, weight,
                 complete_reps, partial_reps, rir)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                date,
                sequence,
                exercise_id,
                equipment_id,
                weight,
                reps,
                partial,
                rir
            ],
        )
    };

    assert!(insert(1, squat, 2, 100.0, 5, 0, Some(2)).is_ok());
    assert!(
        insert(1, squat, 2, 100.0, 5, 0, None).is_err(),
        "duplicate date/sequence"
    );
    assert!(
        insert(2, squat, 2, -1.0, 5, 0, None).is_err(),
        "negative weight"
    );
    assert!(insert(3, squat, 2, 100.0, 0, 0, None).is_err(), "no reps");
    assert!(
        insert(4, squat, 2, 100.0, 5, 0, Some(11)).is_err(),
        "RIR above 10"
    );
    assert!(
        insert(5, 999_999, 2, 100.0, 5, 0, None).is_err(),
        "unknown exercise"
    );
    assert!(
        insert(6, squat, 999_999, 100.0, 5, 0, None).is_err(),
        "unknown equipment"
    );

    // Reference rows with history cannot be deleted, and the reserved record
    // cannot be deleted or deactivated even when it is unreferenced.
    assert!(conn
        .execute("DELETE FROM exercises WHERE id = ?1", [squat])
        .is_err());
    let error = conn
        .execute("DELETE FROM equipment WHERE id = ?1", [NO_EQUIPMENT_ID])
        .unwrap_err();
    assert!(error.to_string().contains("reserved"));
    let error = conn
        .execute(
            "UPDATE equipment SET is_active = 0 WHERE id = ?1",
            [NO_EQUIPMENT_ID],
        )
        .unwrap_err();
    assert!(error.to_string().contains("reserved"));

    assert!(conn
        .execute("INSERT INTO equipment (name) VALUES ('barbell')", [])
        .is_err());
    assert!(conn
        .execute(
            "INSERT INTO exercises (name, normalized_name, category)
             VALUES ('Bad Category', 'bad category', 'Cardio')",
            [],
        )
        .is_err());
}

#[test]
fn library_services_enforce_normalization_and_inactive_references() {
    let (_dir, mut db) = open();
    let squat = seeded_exercise(&db, "Barbell Back Squat");

    let duplicate = library_service::create_exercise(
        db.connection_mut(),
        "  barbell   back squat ",
        "",
        ExerciseCategoryInput::Other,
        &[],
    )
    .unwrap_err();
    assert!(duplicate.to_string().contains("already exists"));

    let count: i64 = db
        .connection()
        .query_row("SELECT COUNT(*) FROM exercises", [], |row| row.get(0))
        .unwrap();
    let squat_rows: i64 = db
        .connection()
        .query_row(
            "SELECT COUNT(*) FROM exercises WHERE lower(trim(name)) = 'barbell back squat'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(squat_rows, 1);
    assert!(count > 1);

    let quadriceps = seeded_muscle(&db, "Quadriceps");
    let duplicate_assignment = library_service::create_exercise(
        db.connection_mut(),
        "Duplicate Assignment Test",
        "",
        ExerciseCategoryInput::Compound,
        &[
            MuscleAssignment {
                muscle_group_id: quadriceps,
                role: MuscleRole::Primary,
            },
            MuscleAssignment {
                muscle_group_id: quadriceps,
                role: MuscleRole::Secondary,
            },
        ],
    )
    .unwrap_err();
    assert!(duplicate_assignment
        .to_string()
        .contains("once per exercise"));

    // Deactivating a reference record is allowed; using it for a *new* set is not.
    library_service::set_exercise_active(db.connection(), squat, false).unwrap();
    let day = today_local() - Duration::days(1);
    let error = workout_service::add_set(db.connection_mut(), &set_input(day, squat, 100.0, 5))
        .unwrap_err();
    assert!(error.to_string().contains("inactive"));

    assert!(
        library_service::set_equipment_active(db.connection(), NO_EQUIPMENT_ID, false).is_err()
    );
    let error =
        library_service::update_equipment(db.connection(), NO_EQUIPMENT_ID, "Bodyweight", "", true)
            .unwrap_err();
    assert!(error.to_string().contains("reserved"));
}

#[test]
fn body_measurements_are_append_only_and_bmi_needs_one_snapshot() {
    let (_dir, db) = open();
    let day_one = today_local() - Duration::days(5);
    let day_two = today_local() - Duration::days(3);
    let day_three = today_local() - Duration::days(1);

    let weight_only = NewBodyMeasurement {
        measured_at: day_one,
        weight_kg: Some(80.0),
        ..Default::default()
    };
    profile_service::add_measurement(db.connection(), &weight_only).unwrap();
    let height_only = NewBodyMeasurement {
        measured_at: day_two,
        height_cm: Some(180.0),
        ..Default::default()
    };
    profile_service::add_measurement(db.connection(), &height_only).unwrap();
    let both = NewBodyMeasurement {
        measured_at: day_three,
        weight_kg: Some(82.0),
        height_cm: Some(181.0),
        body_fat_percentage: Some(18.0),
        ..Default::default()
    };
    profile_service::add_measurement(db.connection(), &both).unwrap();

    let history = profile_service::measurement_history(db.connection()).unwrap();
    assert_eq!(
        history.len(),
        3,
        "measurements are appended, never replaced"
    );
    assert_eq!(
        history
            .iter()
            .find(|measurement| measurement.measured_at == day_one)
            .unwrap()
            .bmi(),
        None,
        "weight without height in the same snapshot has no BMI"
    );
    assert_eq!(
        history
            .iter()
            .find(|measurement| measurement.measured_at == day_two)
            .unwrap()
            .bmi(),
        None,
        "height without weight in the same snapshot has no BMI"
    );
    let latest = profile_service::latest_measurement(db.connection())
        .unwrap()
        .unwrap();
    assert_eq!(latest.measured_at, day_three);
    assert!((latest.bmi().unwrap() - 82.0 / (1.81 * 1.81)).abs() < 1e-9);

    let metrics = profile_service::latest_metrics(db.connection()).unwrap();
    assert_eq!(metrics.weight_kg, Some(82.0));
    assert_eq!(metrics.height_cm, Some(181.0));
    assert_eq!(metrics.bmi_measured_at, Some(day_three));
    assert!((metrics.bmi.unwrap() - 82.0 / (1.81 * 1.81)).abs() < 1e-9);

    let comparisons = profile_service::comparisons(db.connection()).unwrap();
    let weight = comparisons
        .iter()
        .find(|comparison| {
            comparison.metric == fat::models::body_measurement::MeasurementMetric::Weight
        })
        .unwrap();
    assert_eq!(weight.current, Some(82.0));
    assert_eq!(weight.previous, Some(80.0));
    assert_eq!(weight.current_date, Some(day_three));
    assert_eq!(weight.delta(), Some(2.0));

    let empty = NewBodyMeasurement {
        measured_at: day_three,
        ..Default::default()
    };
    assert!(profile_service::add_measurement(db.connection(), &empty).is_err());
    let future = NewBodyMeasurement {
        measured_at: today_local() + Duration::days(1),
        weight_kg: Some(80.0),
        ..Default::default()
    };
    assert!(profile_service::add_measurement(db.connection(), &future).is_err());
}

#[test]
fn analytics_use_actual_dates_and_attribute_primary_and_secondary_volume() {
    let (_dir, mut db) = open();
    let squat = seeded_exercise(&db, "Barbell Back Squat");
    let bench = seeded_exercise(&db, "Bench Press");
    let day_one = today_local() - Duration::days(9);
    let day_two = today_local() - Duration::days(5);
    let day_three = today_local() - Duration::days(2);

    workout_service::add_set(db.connection_mut(), &set_input(day_one, squat, 100.0, 5)).unwrap();
    workout_service::add_set(db.connection_mut(), &set_input(day_two, bench, 80.0, 8)).unwrap();
    workout_service::add_set(db.connection_mut(), &set_input(day_three, squat, 110.0, 3)).unwrap();

    let report = analytics_service::build_report(
        db.connection(),
        Period::AllTime,
        Some(squat),
        None,
        today_local(),
    )
    .unwrap();
    assert_eq!(report.exercise.total_sets, 2);
    assert_eq!(report.exercise.total_volume, 830.0);
    assert_eq!(
        report
            .exercise
            .weight_progression
            .iter()
            .map(|point| point.date)
            .collect::<Vec<_>>(),
        vec![day_one, day_three],
        "only actual workout dates are plotted"
    );
    assert_eq!(
        report
            .exercise
            .weight_progression
            .iter()
            .map(|point| point.value)
            .collect::<Vec<_>>(),
        vec![100.0, 110.0]
    );

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
    assert_eq!(quads.primary_volume, 830.0);
    assert_eq!(quads.primary_sets, 2);
    assert_eq!(glutes.primary_volume, 0.0);
    assert_eq!(glutes.secondary_volume, 415.0);
    assert_eq!(glutes.secondary_sets, 2);
    assert_eq!(glutes.total_volume_attributed(), 415.0);

    let all = analytics_service::build_report(
        db.connection(),
        Period::AllTime,
        None,
        None,
        today_local(),
    )
    .unwrap();
    let bench_row = all
        .exercise_frequency
        .iter()
        .find(|row| row.label == "Bench Press")
        .unwrap();
    assert_eq!(bench_row.value, 1.0);
    let total_weekly: f64 = all.weekly_volume.iter().map(|point| point.value).sum();
    assert!((total_weekly - (830.0 + 640.0)).abs() < 1e-9);
    assert!(
        all.weekly_volume.len() <= 3,
        "only weeks with records are returned"
    );
}

#[test]
fn migrations_validate_versions_and_backups_restore_working_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("workout.db");
    let mut db = Database::open(&path).unwrap();
    assert_eq!(db.schema_version().unwrap(), 1);
    let audit_rows: i64 = db
        .connection()
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(audit_rows, 1);

    let squat = seeded_exercise(&db, "Barbell Back Squat");
    workout_service::add_set(
        db.connection_mut(),
        &set_input(today_local() - Duration::days(1), squat, 100.0, 5),
    )
    .unwrap();

    let backup = db.backup("integration-test").unwrap();
    assert!(backup.exists());
    let before: i64 = db
        .connection()
        .query_row("SELECT COUNT(*) FROM workout_logs", [], |row| row.get(0))
        .unwrap();
    db.connection()
        .execute("DELETE FROM workout_logs", [])
        .unwrap();
    migrations::restore_validated_backup(db.connection_mut(), &backup).unwrap();
    let after: i64 = db
        .connection()
        .query_row("SELECT COUNT(*) FROM workout_logs", [], |row| row.get(0))
        .unwrap();
    assert_eq!(after, before);
    drop(db);

    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();
    }
    let error = Database::open(&path)
        .err()
        .expect("newer schema should be rejected");
    assert!(
        error.to_string().contains("newer"),
        "unexpected error: {error}"
    );

    let unversioned = dir.path().join("unversioned.db");
    {
        let conn = rusqlite::Connection::open(&unversioned).unwrap();
        conn.execute("CREATE TABLE legacy (id INTEGER PRIMARY KEY)", [])
            .unwrap();
    }
    let error = Database::open(&unversioned)
        .err()
        .expect("unversioned database should be rejected");
    assert!(error.to_string().contains("no schema version"));
}

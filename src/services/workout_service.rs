use chrono::NaiveDate;
use rusqlite::Connection;

use crate::database::repositories::{
    equipment_repository, exercise_repository, workout_repository,
};
use crate::error::{AppError, AppResult};
use crate::models::workout::{
    NewWorkoutSet, UpdateWorkoutSet, WorkoutDay, WorkoutDaySummary, WorkoutSet,
};
use crate::utils::calculations::validate_workout_values;
use crate::utils::date::{format_date, today_local};
use crate::utils::text::optional_text;

/// Validated high-level input for a new set. The UI parses text fields first,
/// but services re-validate every invariant because they are also used by tests
/// and future import tools.
#[derive(Debug, Clone)]
pub struct NewSetInput {
    pub date: NaiveDate,
    pub exercise_id: i64,
    pub equipment_id: i64,
    pub weight: f64,
    pub complete_reps: i64,
    pub partial_reps: i64,
    pub rest_seconds: Option<i64>,
    pub rir: Option<i64>,
    pub extra_info: String,
}

#[derive(Debug, Clone)]
pub struct EditSetInput {
    pub exercise_id: i64,
    pub equipment_id: i64,
    pub weight: f64,
    pub complete_reps: i64,
    pub partial_reps: i64,
    pub rest_seconds: Option<i64>,
    pub rir: Option<i64>,
    pub extra_info: String,
}

fn validate_date(date: NaiveDate) -> AppResult<()> {
    if date > today_local() {
        return Err(AppError::validation(
            "Workouts cannot be logged for a future date",
        ));
    }
    if date < NaiveDate::from_ymd_opt(1900, 1, 1).unwrap() {
        return Err(AppError::validation("Workout date is too far in the past"));
    }
    Ok(())
}

fn check_exercise(conn: &Connection, exercise_id: i64, require_active: bool) -> AppResult<()> {
    let exercise = exercise_repository::get(conn, exercise_id)?;
    if require_active && !exercise.is_active {
        return Err(AppError::validation(format!(
            "'{}' is inactive and cannot be used for a new set",
            exercise.name
        )));
    }
    Ok(())
}

fn check_equipment(conn: &Connection, equipment_id: i64, require_active: bool) -> AppResult<()> {
    let equipment = equipment_repository::get(conn, equipment_id)?;
    if require_active && !equipment.is_active {
        return Err(AppError::validation(format!(
            "'{}' is inactive and cannot be used for a new set",
            equipment.name
        )));
    }
    Ok(())
}

pub fn add_set(conn: &mut Connection, input: &NewSetInput) -> AppResult<i64> {
    validate_date(input.date)?;
    validate_workout_values(
        input.weight,
        input.complete_reps,
        input.partial_reps,
        input.rest_seconds,
        input.rir,
    )?;
    check_exercise(conn, input.exercise_id, true)?;
    check_equipment(conn, input.equipment_id, true)?;
    let extra_info = optional_text(&input.extra_info, "Extra information")?;

    let new_set = NewWorkoutSet {
        date: input.date,
        exercise_id: input.exercise_id,
        equipment_id: input.equipment_id,
        weight: input.weight,
        complete_reps: input.complete_reps,
        partial_reps: input.partial_reps,
        rest_seconds: input.rest_seconds,
        rir: input.rir,
        extra_info,
    };
    workout_repository::create(conn, &new_set)
}

pub fn edit_set(
    conn: &Connection,
    date: NaiveDate,
    sequence_number: i64,
    input: &EditSetInput,
) -> AppResult<()> {
    validate_date(date)?;
    validate_workout_values(
        input.weight,
        input.complete_reps,
        input.partial_reps,
        input.rest_seconds,
        input.rir,
    )?;
    let existing = workout_repository::get(conn, date, sequence_number)?;

    // Deactivated reference records remain valid for historical sets. Only
    // changing to a *different* inactive record is rejected.
    check_exercise(
        conn,
        input.exercise_id,
        input.exercise_id != existing.exercise_id,
    )?;
    check_equipment(
        conn,
        input.equipment_id,
        input.equipment_id != existing.equipment_id,
    )?;

    let extra_info = optional_text(&input.extra_info, "Extra information")?;
    let update = UpdateWorkoutSet {
        exercise_id: input.exercise_id,
        equipment_id: input.equipment_id,
        weight: input.weight,
        complete_reps: input.complete_reps,
        partial_reps: input.partial_reps,
        rest_seconds: input.rest_seconds,
        rir: input.rir,
        extra_info,
    };
    workout_repository::update(conn, date, sequence_number, &update)
}

pub fn delete_set(conn: &Connection, date: NaiveDate, sequence_number: i64) -> AppResult<()> {
    workout_repository::delete(conn, date, sequence_number)
}

pub fn get_set(conn: &Connection, date: NaiveDate, sequence_number: i64) -> AppResult<WorkoutSet> {
    workout_repository::get(conn, date, sequence_number)
}

pub fn workout_for_date(conn: &Connection, date: NaiveDate) -> AppResult<WorkoutDay> {
    Ok(WorkoutDay {
        date,
        sets: workout_repository::list_for_date(conn, date)?,
    })
}

pub fn today_workout(conn: &Connection) -> AppResult<WorkoutDay> {
    workout_for_date(conn, today_local())
}

pub fn history(conn: &Connection) -> AppResult<Vec<WorkoutDaySummary>> {
    workout_repository::list_day_summaries(conn)
}

pub fn set_count(conn: &Connection, date: NaiveDate) -> AppResult<i64> {
    workout_repository::set_count(conn, date)
}

pub fn next_sequence(conn: &Connection, date: NaiveDate) -> AppResult<i64> {
    // Preview only. The repository computes the actual sequence atomically
    // inside the insert transaction.
    let sets = workout_repository::list_for_date(conn, date)?;
    Ok(sets
        .iter()
        .map(|set| set.sequence_number)
        .max()
        .unwrap_or(0)
        + 1)
}

/// Exercise names performed on a date, in first-performed order.
pub fn exercises_performed_today(conn: &Connection) -> AppResult<Vec<String>> {
    let day = today_workout(conn)?;
    Ok(day.exercise_names())
}

pub fn date_label(date: NaiveDate) -> String {
    format_date(date)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::services::library_service;

    fn setup() -> (tempfile::TempDir, Database, i64, i64) {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(dir.path().join("workout.db")).unwrap();
        let exercises = library_service::list_exercises(db.connection(), true, None).unwrap();
        let squat = exercises
            .iter()
            .find(|exercise| exercise.name == "Barbell Back Squat")
            .unwrap()
            .id;
        (dir, db, squat, 1)
    }

    fn input(date: NaiveDate, exercise_id: i64, weight: f64, reps: i64) -> NewSetInput {
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
    fn sequence_continues_across_exercises_and_dates() {
        let (_dir, mut db, squat, _) = setup();
        let other = library_service::list_exercises(db.connection(), true, None)
            .unwrap()
            .into_iter()
            .find(|exercise| exercise.name == "Leg Extension")
            .unwrap();
        let date = today_local();
        let mut first = input(date, squat, 30.0, 6);
        assert_eq!(add_set(db.connection_mut(), &first).unwrap(), 1);
        first.weight = 35.0;
        assert_eq!(add_set(db.connection_mut(), &first).unwrap(), 2);
        let mut second = input(date, other.id, 45.0, 10);
        second.equipment_id = 7;
        assert_eq!(add_set(db.connection_mut(), &second).unwrap(), 3);

        let day = workout_for_date(db.connection(), date).unwrap();
        assert_eq!(day.sets.len(), 3);
        assert_eq!(day.sets[2].sequence_number, 3);
        assert_eq!(day.sets[1].exercise_name, "Barbell Back Squat");
    }

    #[test]
    fn deleting_a_set_does_not_renumber_following_sets() {
        let (_dir, mut db, squat, _) = setup();
        let date = today_local();
        add_set(db.connection_mut(), &input(date, squat, 30.0, 6)).unwrap();
        add_set(db.connection_mut(), &input(date, squat, 35.0, 6)).unwrap();
        add_set(db.connection_mut(), &input(date, squat, 40.0, 6)).unwrap();
        delete_set(db.connection(), date, 2).unwrap();
        let day = workout_for_date(db.connection(), date).unwrap();
        let sequences: Vec<i64> = day.sets.iter().map(|set| set.sequence_number).collect();
        assert_eq!(sequences, vec![1, 3]);
        assert_eq!(next_sequence(db.connection(), date).unwrap(), 4);
    }

    #[test]
    fn invalid_values_are_rejected() {
        let (_dir, mut db, squat, _) = setup();
        let date = today_local();
        let mut invalid = input(date, squat, -1.0, 5);
        assert!(add_set(db.connection_mut(), &invalid).is_err());
        invalid = input(date, squat, 20.0, 5);
        invalid.rir = Some(11);
        assert!(add_set(db.connection_mut(), &invalid).is_err());
        invalid = input(date, squat, 20.0, 0);
        assert!(add_set(db.connection_mut(), &invalid).is_err());
    }

    #[test]
    fn inactive_exercise_cannot_be_used_for_new_sets_but_existing_set_can_be_edited() {
        let (_dir, mut db, squat, _) = setup();
        let date = today_local();
        add_set(db.connection_mut(), &input(date, squat, 30.0, 6)).unwrap();
        library_service::set_exercise_active(db.connection(), squat, false).unwrap();
        assert!(add_set(db.connection_mut(), &input(date, squat, 30.0, 6)).is_err());
        let edit = EditSetInput {
            exercise_id: squat,
            equipment_id: 2,
            weight: 32.5,
            complete_reps: 6,
            partial_reps: 1,
            rest_seconds: None,
            rir: None,
            extra_info: "felt strong".into(),
        };
        edit_set(db.connection(), date, 1, &edit).unwrap();
        let set = get_set(db.connection(), date, 1).unwrap();
        assert_eq!(set.weight, 32.5);
        assert_eq!(set.partial_reps, 1);
    }

    #[test]
    fn volume_uses_effective_reps() {
        let (_dir, mut db, squat, _) = setup();
        let date = today_local();
        let mut set = input(date, squat, 30.0, 6);
        set.partial_reps = 1;
        add_set(db.connection_mut(), &set).unwrap();
        let day = workout_for_date(db.connection(), date).unwrap();
        assert_eq!(day.sets[0].volume(), 195.0);
        assert_eq!(day.total_volume(), 195.0);
    }

    #[test]
    fn future_workouts_are_rejected() {
        let (_dir, mut db, squat, _) = setup();
        let tomorrow = today_local() + chrono::Duration::days(1);
        assert!(add_set(db.connection_mut(), &input(tomorrow, squat, 30.0, 6)).is_err());
    }
}

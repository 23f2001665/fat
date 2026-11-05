use rusqlite::Connection;

use crate::database::repositories::exercise_repository::ExerciseWrite;
use crate::database::repositories::{equipment_repository, exercise_repository, muscle_repository};
use crate::error::{AppError, AppResult};
use crate::models::equipment::{Equipment, NO_EQUIPMENT_ID};
use crate::models::exercise::{
    Exercise, ExerciseCategory, ExerciseCategoryInput, MuscleAssignment, MuscleRole,
};
use crate::models::muscle_group::MuscleGroup;
use crate::utils::text::{clean_name, normalize_name, optional_text, required_name};

fn category_from_input(input: ExerciseCategoryInput) -> ExerciseCategory {
    input.into_category()
}

fn validate_assignments(
    conn: &Connection,
    assignments: &[MuscleAssignment],
) -> AppResult<Vec<MuscleAssignment>> {
    let mut seen = std::collections::HashSet::new();
    let mut validated = Vec::with_capacity(assignments.len());
    for assignment in assignments {
        if !seen.insert(assignment.muscle_group_id) {
            return Err(AppError::validation(
                "A muscle group can only be assigned once per exercise",
            ));
        }
        let muscle = muscle_repository::get(conn, assignment.muscle_group_id)?;
        if !muscle.is_active {
            return Err(AppError::validation(format!(
                "'{}' is inactive and cannot be assigned",
                muscle.name
            )));
        }
        validated.push(MuscleAssignment {
            muscle_group_id: assignment.muscle_group_id,
            role: assignment.role,
        });
    }
    Ok(validated)
}

pub fn list_exercises(
    conn: &Connection,
    include_inactive: bool,
    search: Option<&str>,
) -> AppResult<Vec<Exercise>> {
    exercise_repository::list(conn, include_inactive, search)
}

pub fn get_exercise(conn: &Connection, id: i64) -> AppResult<Exercise> {
    exercise_repository::get(conn, id)
}

/// Creates a custom exercise. Name comparison is case- and whitespace-insensitive.
pub fn create_exercise(
    conn: &mut Connection,
    name: &str,
    description: &str,
    category: ExerciseCategoryInput,
    muscles: &[MuscleAssignment],
) -> AppResult<i64> {
    let name = required_name(name, "Exercise name")?;
    let normalized = normalize_name(&name);
    if let Some(existing) = exercise_repository::find_by_normalized_name(conn, &normalized)? {
        return Err(AppError::conflict(format!(
            "An exercise named '{}' already exists",
            existing.name
        )));
    }
    let description = optional_text(description, "Description")?;
    let muscles = validate_assignments(conn, muscles)?;
    let write = ExerciseWrite {
        name: &name,
        normalized_name: &normalized,
        description: description.as_deref(),
        category: category_from_input(category),
        is_active: true,
        muscles: &muscles,
    };
    exercise_repository::create(conn, &write)
}

pub fn update_exercise(
    conn: &mut Connection,
    id: i64,
    name: &str,
    description: &str,
    category: ExerciseCategoryInput,
    is_active: bool,
    muscles: &[MuscleAssignment],
) -> AppResult<()> {
    let name = required_name(name, "Exercise name")?;
    let normalized = normalize_name(&name);
    if let Some(existing) = exercise_repository::find_by_normalized_name(conn, &normalized)? {
        if existing.id != id {
            return Err(AppError::conflict(format!(
                "An exercise named '{}' already exists",
                existing.name
            )));
        }
    }
    let description = optional_text(description, "Description")?;
    let muscles = validate_assignments(conn, muscles)?;
    let write = ExerciseWrite {
        name: &name,
        normalized_name: &normalized,
        description: description.as_deref(),
        category: category_from_input(category),
        is_active,
        muscles: &muscles,
    };
    exercise_repository::update(conn, id, &write)
}

pub fn set_exercise_active(conn: &Connection, id: i64, active: bool) -> AppResult<()> {
    exercise_repository::set_active(conn, id, active)
}

pub fn list_muscle_groups(
    conn: &Connection,
    include_inactive: bool,
    search: Option<&str>,
) -> AppResult<Vec<MuscleGroup>> {
    muscle_repository::list(conn, include_inactive, search)
}

pub fn get_muscle_group(conn: &Connection, id: i64) -> AppResult<MuscleGroup> {
    muscle_repository::get(conn, id)
}

pub fn create_muscle_group(conn: &Connection, name: &str, description: &str) -> AppResult<i64> {
    let name = required_name(name, "Muscle group name")?;
    if muscle_repository::name_exists(conn, &name, None)? {
        return Err(AppError::conflict(
            "A muscle group with that name already exists",
        ));
    }
    let description = optional_text(description, "Description")?;
    muscle_repository::create(conn, &name, description.as_deref())
}

pub fn update_muscle_group(
    conn: &Connection,
    id: i64,
    name: &str,
    description: &str,
    is_active: bool,
) -> AppResult<()> {
    let name = required_name(name, "Muscle group name")?;
    if muscle_repository::name_exists(conn, &name, Some(id))? {
        return Err(AppError::conflict(
            "A muscle group with that name already exists",
        ));
    }
    let existing = muscle_repository::get(conn, id)?;
    if existing.is_active && !is_active {
        let count = muscle_repository::association_count(conn, id)?;
        if count > 0 {
            return Err(AppError::validation(
                "This muscle group is used by exercises and cannot be deactivated. \
                 Remove it from those exercise definitions first.",
            ));
        }
    }
    let description = optional_text(description, "Description")?;
    muscle_repository::update(conn, id, &name, description.as_deref(), is_active)
}

pub fn set_muscle_group_active(conn: &Connection, id: i64, active: bool) -> AppResult<()> {
    let group = muscle_repository::get(conn, id)?;
    update_muscle_group(
        conn,
        id,
        &group.name,
        group.description.as_deref().unwrap_or(""),
        active,
    )
}

pub fn list_equipment(
    conn: &Connection,
    include_inactive: bool,
    search: Option<&str>,
) -> AppResult<Vec<Equipment>> {
    equipment_repository::list(conn, include_inactive, search)
}

pub fn get_equipment(conn: &Connection, id: i64) -> AppResult<Equipment> {
    equipment_repository::get(conn, id)
}

pub fn create_equipment(conn: &Connection, name: &str, category: &str) -> AppResult<i64> {
    let name = required_name(name, "Equipment name")?;
    if equipment_repository::name_exists(conn, &name, None)? {
        return Err(AppError::conflict(
            "Equipment with that name already exists",
        ));
    }
    let category = optional_text(category, "Category")?;
    equipment_repository::create(conn, &name, category.as_deref())
}

pub fn update_equipment(
    conn: &Connection,
    id: i64,
    name: &str,
    category: &str,
    is_active: bool,
) -> AppResult<()> {
    let name = required_name(name, "Equipment name")?;
    if id == NO_EQUIPMENT_ID && (!is_active || name != "No Equipment") {
        return Err(AppError::validation(
            "The reserved No Equipment record must remain active and keep its name",
        ));
    }
    if equipment_repository::name_exists(conn, &name, Some(id))? {
        return Err(AppError::conflict(
            "Equipment with that name already exists",
        ));
    }
    let category = optional_text(category, "Category")?;
    equipment_repository::update(conn, id, &name, category.as_deref(), is_active)
}

pub fn set_equipment_active(conn: &Connection, id: i64, active: bool) -> AppResult<()> {
    if id == NO_EQUIPMENT_ID && !active {
        return Err(AppError::validation(
            "No Equipment is reserved and cannot be deactivated",
        ));
    }
    equipment_repository::set_active(conn, id, active)
}

/// Quick-create helpers used by pickers so users need not leave a workout flow.
pub fn quick_create_exercise(conn: &mut Connection, name: &str) -> AppResult<i64> {
    create_exercise(conn, name, "", ExerciseCategoryInput::Other, &[])
}

pub fn quick_create_muscle_group(conn: &Connection, name: &str) -> AppResult<i64> {
    create_muscle_group(conn, name, "")
}

pub fn quick_create_equipment(conn: &Connection, name: &str) -> AppResult<i64> {
    create_equipment(conn, name, "")
}

/// Used by the UI to render a primary/secondary marker.
pub fn role_label(role: MuscleRole) -> &'static str {
    role.as_str()
}

/// Ensures a display name is trimmed even when it was originally typed with
/// surrounding spaces.
pub fn display_name(value: &str) -> String {
    clean_name(value)
}

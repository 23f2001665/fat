use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

use crate::database::connection::ensure_foreign_keys;
use crate::error::{AppError, AppResult};
use crate::utils::date::now_timestamp;
use crate::utils::text::normalize_name;

struct SeedExercise {
    name: &'static str,
    category: &'static str,
    muscles: &'static [(&'static str, &'static str)],
}

const MUSCLE_GROUPS: &[(&str, &str)] = &[
    ("Chest", "Pectoralis major"),
    ("Upper Chest", "Clavicular pectoralis"),
    ("Triceps", "Elbow extensors"),
    ("Biceps", "Elbow flexors"),
    ("Brachialis", "Elbow flexor beneath the biceps"),
    ("Forearms", "Wrist and grip musculature"),
    ("Front Delts", "Anterior deltoid"),
    ("Side Delts", "Lateral deltoid"),
    ("Rear Delts", "Posterior deltoid"),
    ("Upper Back", "Traps, rhomboids and lats"),
    ("Lower Back", "Erector spinae"),
    ("Quadriceps", "Knee extensors"),
    ("Hamstrings", "Knee flexors and hip extensors"),
    ("Glutes", "Hip extensors"),
    ("Calves", "Ankle plantar flexors"),
    ("Abs", "Trunk flexors and anti-extension"),
];

const EQUIPMENT: &[(i64, &str, &str)] = &[
    (2, "Barbell", "Free weights"),
    (3, "Dumbbell", "Free weights"),
    (4, "Cable Machine", "Machines"),
    (5, "Smith Machine", "Machines"),
    (6, "Leg Press Machine", "Machines"),
    (7, "Leg Extension Machine", "Machines"),
    (8, "Resistance Band", "Portable"),
    (9, "Other", "Other"),
];

const EXERCISES: &[SeedExercise] = &[
    SeedExercise {
        name: "Barbell Back Squat",
        category: "Compound",
        muscles: &[
            ("Quadriceps", "Primary"),
            ("Glutes", "Secondary"),
            ("Lower Back", "Secondary"),
            ("Abs", "Secondary"),
        ],
    },
    SeedExercise {
        name: "Leg Press",
        category: "Compound",
        muscles: &[
            ("Quadriceps", "Primary"),
            ("Glutes", "Secondary"),
            ("Hamstrings", "Secondary"),
        ],
    },
    SeedExercise {
        name: "Leg Extension",
        category: "Isolation",
        muscles: &[("Quadriceps", "Primary")],
    },
    SeedExercise {
        name: "Romanian Deadlift",
        category: "Compound",
        muscles: &[
            ("Hamstrings", "Primary"),
            ("Glutes", "Primary"),
            ("Lower Back", "Secondary"),
        ],
    },
    SeedExercise {
        name: "Bench Press",
        category: "Compound",
        muscles: &[
            ("Chest", "Primary"),
            ("Triceps", "Secondary"),
            ("Front Delts", "Secondary"),
        ],
    },
    SeedExercise {
        name: "Incline Dumbbell Press",
        category: "Compound",
        muscles: &[
            ("Upper Chest", "Primary"),
            ("Triceps", "Secondary"),
            ("Front Delts", "Secondary"),
        ],
    },
    SeedExercise {
        name: "Overhead Press",
        category: "Compound",
        muscles: &[
            ("Front Delts", "Primary"),
            ("Triceps", "Secondary"),
            ("Side Delts", "Secondary"),
        ],
    },
    SeedExercise {
        name: "Lateral Raise",
        category: "Isolation",
        muscles: &[("Side Delts", "Primary")],
    },
    SeedExercise {
        name: "Pull Up",
        category: "Bodyweight",
        muscles: &[
            ("Upper Back", "Primary"),
            ("Biceps", "Secondary"),
            ("Forearms", "Secondary"),
        ],
    },
    SeedExercise {
        name: "Barbell Row",
        category: "Compound",
        muscles: &[
            ("Upper Back", "Primary"),
            ("Biceps", "Secondary"),
            ("Rear Delts", "Secondary"),
            ("Lower Back", "Secondary"),
        ],
    },
    SeedExercise {
        name: "Lat Pulldown",
        category: "Compound",
        muscles: &[("Upper Back", "Primary"), ("Biceps", "Secondary")],
    },
    SeedExercise {
        name: "Face Pull",
        category: "Isolation",
        muscles: &[("Rear Delts", "Primary"), ("Upper Back", "Secondary")],
    },
    SeedExercise {
        name: "Barbell Curl",
        category: "Isolation",
        muscles: &[("Biceps", "Primary"), ("Forearms", "Secondary")],
    },
    SeedExercise {
        name: "Hammer Curl",
        category: "Isolation",
        muscles: &[
            ("Brachialis", "Primary"),
            ("Biceps", "Secondary"),
            ("Forearms", "Secondary"),
        ],
    },
    SeedExercise {
        name: "Triceps Pushdown",
        category: "Isolation",
        muscles: &[("Triceps", "Primary")],
    },
    SeedExercise {
        name: "Skull Crusher",
        category: "Isolation",
        muscles: &[("Triceps", "Primary")],
    },
    SeedExercise {
        name: "Dips",
        category: "Bodyweight",
        muscles: &[
            ("Chest", "Primary"),
            ("Triceps", "Secondary"),
            ("Front Delts", "Secondary"),
        ],
    },
    SeedExercise {
        name: "Standing Calf Raise",
        category: "Isolation",
        muscles: &[("Calves", "Primary")],
    },
    SeedExercise {
        name: "Hanging Leg Raise",
        category: "Bodyweight",
        muscles: &[("Abs", "Primary"), ("Forearms", "Secondary")],
    },
    SeedExercise {
        name: "Plank",
        category: "Bodyweight",
        muscles: &[("Abs", "Primary"), ("Lower Back", "Secondary")],
    },
    SeedExercise {
        name: "Hip Thrust",
        category: "Compound",
        muscles: &[("Glutes", "Primary"), ("Hamstrings", "Secondary")],
    },
    SeedExercise {
        name: "Walking Lunge",
        category: "Compound",
        muscles: &[("Quadriceps", "Primary"), ("Glutes", "Secondary")],
    },
    SeedExercise {
        name: "Cable Fly",
        category: "Isolation",
        muscles: &[("Chest", "Primary"), ("Front Delts", "Secondary")],
    },
    SeedExercise {
        name: "Rear Delt Fly",
        category: "Isolation",
        muscles: &[("Rear Delts", "Primary"), ("Upper Back", "Secondary")],
    },
];

const M_KEY: &str = "seed.muscle_groups.v1";
const E_KEY: &str = "seed.equipment.v1";
const X_KEY: &str = "seed.exercises.v1";
const U_KEY: &str = "seed.local_user.v1";

fn marker_present(conn: &Connection, key: &str) -> AppResult<bool> {
    let exists: i64 = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM app_metadata WHERE key = ?1)",
        [key],
        |row| row.get(0),
    )?;
    Ok(exists == 1)
}

fn set_marker(conn: &Connection, key: &str) -> AppResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO app_metadata (key, value) VALUES (?1, ?2)",
        params![key, now_timestamp()],
    )?;
    Ok(())
}

/// Inserts the predefined catalogue and local user.
///
/// This runs on every start and is idempotent. Marker rows ensure that a user
/// renaming or deactivating a seeded record cannot cause it to be recreated as
/// a duplicate on the next launch. Future catalogue revisions use new markers.
pub fn seed(conn: &mut Connection) -> AppResult<()> {
    ensure_foreign_keys(conn)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    // The reserved No Equipment row is mandatory and is verified on every start.
    let reserved: Option<String> = tx
        .query_row("SELECT name FROM equipment WHERE id = 1", [], |row| {
            row.get(0)
        })
        .optional()?;
    match reserved.as_deref() {
        Some("No Equipment") => {}
        Some(other) => {
            return Err(AppError::conflict(format!(
                "Equipment id 1 must be the reserved 'No Equipment' record, found '{other}'"
            )));
        }
        None => {
            tx.execute(
                "INSERT INTO equipment (id, name, category, is_active)
                 VALUES (1, 'No Equipment', 'Reserved', 1)",
                [],
            )?;
        }
    }

    if !marker_present(&tx, M_KEY)? {
        for (name, description) in MUSCLE_GROUPS {
            tx.execute(
                "INSERT OR IGNORE INTO muscle_groups (name, description, is_active)
                 VALUES (?1, ?2, 1)",
                params![name, description],
            )?;
        }
        set_marker(&tx, M_KEY)?;
    }

    if !marker_present(&tx, E_KEY)? {
        for (id, name, category) in EQUIPMENT {
            tx.execute(
                "INSERT OR IGNORE INTO equipment (id, name, category, is_active)
                 VALUES (?1, ?2, ?3, 1)",
                params![id, name, category],
            )?;
        }
        set_marker(&tx, E_KEY)?;
    }

    if !marker_present(&tx, X_KEY)? {
        for exercise in EXERCISES {
            let normalized = normalize_name(exercise.name);
            tx.execute(
                "INSERT OR IGNORE INTO exercises
                    (name, normalized_name, description, category, is_active)
                 VALUES (?1, ?2, NULL, ?3, 1)",
                params![exercise.name, normalized, exercise.category],
            )?;
            let exercise_id: i64 = tx.query_row(
                "SELECT id FROM exercises WHERE normalized_name = ?1",
                [&normalized],
                |row| row.get(0),
            )?;
            for (muscle_name, role) in exercise.muscles {
                if let Some(muscle_id) = tx
                    .query_row(
                        "SELECT id FROM muscle_groups WHERE lower(trim(name)) = lower(?1)",
                        [muscle_name],
                        |row| row.get::<_, i64>(0),
                    )
                    .optional()?
                {
                    tx.execute(
                        "INSERT OR IGNORE INTO exercise_muscles
                            (exercise_id, muscle_group_id, role)
                         VALUES (?1, ?2, ?3)",
                        params![exercise_id, muscle_id, role],
                    )?;
                }
            }
        }
        set_marker(&tx, X_KEY)?;
    }

    if !marker_present(&tx, U_KEY)? {
        let user_exists: i64 = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM users WHERE id = 1)",
            [],
            |row| row.get(0),
        )?;
        if user_exists == 0 {
            let now = now_timestamp();
            tx.execute(
                "INSERT INTO users (id, name, height_cm, date_of_birth, created_at, updated_at)
                 VALUES (1, 'Athlete', NULL, NULL, ?1, ?1)",
                params![now],
            )?;
        }
        set_marker(&tx, U_KEY)?;
    }

    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{connection::Database, migrations};

    fn test_db() -> (tempfile::TempDir, Database) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        (dir, Database::open(&path).unwrap())
    }

    #[test]
    fn seeding_twice_does_not_duplicate_reference_data() {
        let (_dir, mut db) = test_db();
        seed(db.connection_mut()).unwrap();
        seed(db.connection_mut()).unwrap();

        let muscles: i64 = db
            .connection()
            .query_row("SELECT COUNT(*) FROM muscle_groups", [], |r| r.get(0))
            .unwrap();
        let exercises: i64 = db
            .connection()
            .query_row("SELECT COUNT(*) FROM exercises", [], |r| r.get(0))
            .unwrap();
        let equipment: i64 = db
            .connection()
            .query_row("SELECT COUNT(*) FROM equipment", [], |r| r.get(0))
            .unwrap();
        assert_eq!(muscles, MUSCLE_GROUPS.len() as i64);
        assert_eq!(equipment, EQUIPMENT.len() as i64 + 1);
        assert_eq!(exercises, EXERCISES.len() as i64);
        assert_eq!(migrations::schema_version(db.connection()).unwrap(), 1);
    }

    #[test]
    fn reserved_equipment_is_always_available() {
        let (_dir, db) = test_db();
        let (name, active): (String, i64) = db
            .connection()
            .query_row(
                "SELECT name, is_active FROM equipment WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(name, "No Equipment");
        assert_eq!(active, 1);
    }

    #[test]
    fn reserved_equipment_cannot_be_deleted_or_deactivated() {
        let (_dir, db) = test_db();
        assert!(db
            .connection()
            .execute("DELETE FROM equipment WHERE id = 1", [])
            .is_err());
        assert!(db
            .connection()
            .execute("UPDATE equipment SET is_active = 0 WHERE id = 1", [])
            .is_err());
    }
}

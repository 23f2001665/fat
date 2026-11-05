//! SQLite schema definitions and version metadata.
//!
//! Schema versioning is intentionally independent from the application version.
//! `PRAGMA user_version` is the authoritative schema version; the
//! `schema_migrations` table provides an audit trail of applied migrations.

/// Current schema version. Increment this whenever a migration is added.
pub const SCHEMA_VERSION: i64 = 1;

pub const REQUIRED_TABLES: &[&str] = &[
    "app_metadata",
    "body_measurements",
    "equipment",
    "exercise_muscles",
    "exercises",
    "muscle_groups",
    "schema_migrations",
    "users",
    "workout_logs",
];

/// Initial schema (migration 1).
///
/// Foreign keys are declared for every reference. `ON DELETE RESTRICT` protects
/// historical records and reference data. Check constraints mirror the
/// validation performed in services; the database is the final line of defence.
pub const SCHEMA_V1: &str = r#"
CREATE TABLE schema_migrations (
    version     INTEGER PRIMARY KEY,
    description TEXT NOT NULL,
    applied_at  TEXT NOT NULL
);

CREATE TABLE app_metadata (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE users (
    id            INTEGER PRIMARY KEY,
    name          TEXT NOT NULL,
    height_cm     REAL CHECK (height_cm IS NULL OR height_cm > 0),
    date_of_birth TEXT,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

CREATE TABLE muscle_groups (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    is_active   INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1))
);
CREATE UNIQUE INDEX muscle_groups_name_nocase
    ON muscle_groups (lower(trim(name)));

CREATE TABLE equipment (
    id        INTEGER PRIMARY KEY,
    name      TEXT NOT NULL UNIQUE,
    category  TEXT,
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1))
);
CREATE UNIQUE INDEX equipment_name_nocase
    ON equipment (lower(trim(name)));

CREATE TABLE exercises (
    id              INTEGER PRIMARY KEY,
    name            TEXT NOT NULL,
    normalized_name TEXT NOT NULL UNIQUE,
    description     TEXT,
    category        TEXT NOT NULL DEFAULT 'Other'
                    CHECK (category IN ('Compound', 'Isolation', 'Bodyweight', 'Other')),
    is_active       INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1))
);

CREATE TABLE exercise_muscles (
    exercise_id     INTEGER NOT NULL,
    muscle_group_id INTEGER NOT NULL,
    role            TEXT NOT NULL CHECK (role IN ('Primary', 'Secondary')),
    PRIMARY KEY (exercise_id, muscle_group_id),
    FOREIGN KEY (exercise_id) REFERENCES exercises (id) ON DELETE RESTRICT,
    FOREIGN KEY (muscle_group_id) REFERENCES muscle_groups (id) ON DELETE RESTRICT
);
CREATE INDEX exercise_muscles_muscle_idx ON exercise_muscles (muscle_group_id);

CREATE TABLE workout_logs (
    date             TEXT NOT NULL
                     CHECK (date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
    sequence_number  INTEGER NOT NULL CHECK (sequence_number > 0),
    exercise_id      INTEGER NOT NULL,
    equipment_id     INTEGER NOT NULL,
    weight           REAL NOT NULL DEFAULT 0 CHECK (weight >= 0),
    complete_reps    INTEGER NOT NULL DEFAULT 0 CHECK (complete_reps >= 0),
    partial_reps     INTEGER NOT NULL DEFAULT 0 CHECK (partial_reps >= 0),
    rest_seconds     INTEGER CHECK (rest_seconds IS NULL OR rest_seconds >= 0),
    rir              INTEGER CHECK (rir IS NULL OR (rir >= 0 AND rir <= 10)),
    extra_info       TEXT,
    PRIMARY KEY (date, sequence_number),
    FOREIGN KEY (exercise_id) REFERENCES exercises (id) ON DELETE RESTRICT,
    FOREIGN KEY (equipment_id) REFERENCES equipment (id) ON DELETE RESTRICT,
    CHECK (complete_reps + partial_reps > 0)
);
CREATE INDEX workout_logs_exercise_idx ON workout_logs (exercise_id);
CREATE INDEX workout_logs_equipment_idx ON workout_logs (equipment_id);
CREATE INDEX workout_logs_date_idx ON workout_logs (date);

CREATE TABLE body_measurements (
    id                   INTEGER PRIMARY KEY,
    user_id              INTEGER NOT NULL,
    measured_at          TEXT NOT NULL
                         CHECK (measured_at GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
    weight_kg            REAL CHECK (weight_kg IS NULL OR weight_kg > 0),
    height_cm            REAL CHECK (height_cm IS NULL OR height_cm > 0),
    body_fat_percentage  REAL CHECK (body_fat_percentage IS NULL
                                    OR (body_fat_percentage >= 0 AND body_fat_percentage <= 100)),
    waist_cm             REAL CHECK (waist_cm IS NULL OR waist_cm >= 0),
    chest_cm             REAL CHECK (chest_cm IS NULL OR chest_cm >= 0),
    hip_cm               REAL CHECK (hip_cm IS NULL OR hip_cm >= 0),
    arm_cm               REAL CHECK (arm_cm IS NULL OR arm_cm >= 0),
    thigh_cm             REAL CHECK (thigh_cm IS NULL OR thigh_cm >= 0),
    notes                TEXT,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE RESTRICT
);
CREATE INDEX body_measurements_user_date_idx
    ON body_measurements (user_id, measured_at);

-- The reserved record represents exercises that need no external equipment.
CREATE TRIGGER equipment_no_equipment_delete
BEFORE DELETE ON equipment
WHEN OLD.id = 1
BEGIN
    SELECT RAISE(ABORT, 'No Equipment is reserved and cannot be deleted');
END;

CREATE TRIGGER equipment_no_equipment_deactivate
BEFORE UPDATE ON equipment
WHEN OLD.id = 1
     AND (NEW.id <> 1 OR NEW.is_active <> 1 OR NEW.name <> 'No Equipment')
BEGIN
    SELECT RAISE(ABORT, 'No Equipment is reserved and must stay active');
END;

-- A workout set can never point at a blank exercise name: references are IDs.
-- RIR and rest have no defaults because "not recorded" (NULL) differs from zero.
"#;

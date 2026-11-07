from __future__ import annotations

import sqlite3

from backend.application import create_app
from backend.config import Config
from backend.extensions import db


LEGACY_SCHEMA = """
CREATE TABLE users (
    id INTEGER NOT NULL,
    name VARCHAR(120) NOT NULL,
    height_cm FLOAT,
    date_of_birth DATE,
    created_at VARCHAR(64) NOT NULL,
    updated_at VARCHAR(64) NOT NULL,
    PRIMARY KEY (id),
    CONSTRAINT user_height_positive CHECK (height_cm IS NULL OR height_cm > 0)
);

CREATE TABLE workout_logs (
    date DATE NOT NULL,
    sequence_number INTEGER NOT NULL,
    exercise_id INTEGER NOT NULL,
    equipment_id INTEGER NOT NULL,
    weight FLOAT NOT NULL,
    complete_reps INTEGER NOT NULL,
    partial_reps INTEGER NOT NULL,
    rest_seconds INTEGER,
    rir INTEGER,
    extra_info TEXT,
    PRIMARY KEY (date, sequence_number),
    CONSTRAINT workout_weight_non_negative CHECK (weight >= 0),
    CONSTRAINT workout_complete_reps_non_negative CHECK (complete_reps >= 0),
    CONSTRAINT workout_partial_reps_non_negative CHECK (partial_reps >= 0),
    CONSTRAINT workout_has_reps CHECK (complete_reps + partial_reps > 0),
    CONSTRAINT workout_rest_non_negative CHECK (rest_seconds IS NULL OR rest_seconds >= 0),
    CONSTRAINT workout_rir_range CHECK (rir IS NULL OR (rir >= 0 AND rir <= 10))
);

CREATE INDEX workout_logs_date_idx ON workout_logs (date);
CREATE INDEX workout_logs_exercise_idx ON workout_logs (exercise_id);
CREATE INDEX workout_logs_equipment_idx ON workout_logs (equipment_id);

INSERT INTO users (id, name, created_at, updated_at)
VALUES (1, 'Legacy Athlete', '2026-01-01T08:00:00', '2026-01-01T08:00:00');

INSERT INTO workout_logs
    (date, sequence_number, exercise_id, equipment_id, weight, complete_reps,
     partial_reps, rest_seconds, rir, extra_info)
VALUES
    ('2026-01-02', 1, 1, 2, 100, 5, 1, 120, 2, 'legacy row');
"""

OVERRIDES = {
    "TESTING": True,
    "SEED_DATABASE": True,
    "CREATE_SCHEMA": True,
    "SECRET_KEY": "migration-secret",
    "ADMIN_USERNAME": "admin",
    "ADMIN_PASSWORD": "admin123",
    "DEFAULT_USERNAME": "athlete",
    "DEFAULT_PASSWORD": "athlete123",
}


def _create_legacy_database(path):
    connection = sqlite3.connect(path)
    connection.executescript(LEGACY_SCHEMA)
    connection.commit()
    connection.close()


def test_existing_database_is_migrated_without_data_loss(tmp_path):
    database = tmp_path / "legacy.db"
    _create_legacy_database(database)
    overrides = {
        **OVERRIDES,
        "SQLALCHEMY_DATABASE_URI": f"sqlite:///{database}",
        "FRONTEND_DIST": tmp_path / "frontend-dist",
    }

    app = create_app(Config, overrides)
    client = app.test_client()

    anonymous = client.get("/api/workouts")
    assert anonymous.status_code == 401

    login = client.post(
        "/api/auth/login",
        json={"username": "admin", "password": "admin123"},
    )
    assert login.status_code == 200

    profile = client.get("/api/profile").get_json()
    assert profile["user"]["name"] == "Legacy Athlete"
    assert profile["user"]["role"] == "admin"

    day = client.get(
        "/api/workouts", query_string={"date": "2026-01-02"}
    ).get_json()
    assert day["set_count"] == 1
    assert day["sets"][0]["sequence_number"] == 1
    assert day["sets"][0]["weight"] == 100
    assert day["sets"][0]["exercise_name"] == "Barbell Back Squat"
    assert day["sets"][0]["extra_info"] == "legacy row"

    users = client.get("/api/admin/users").get_json()["users"]
    assert {row["username"] for row in users} == {"admin", "athlete"}

    connection = sqlite3.connect(database)
    assert connection.execute(
        "SELECT count(*) FROM workout_logs WHERE user_id = 1"
    ).fetchone()[0] == 1
    assert connection.execute(
        "SELECT count(*) FROM sqlite_master WHERE name = 'workout_logs_legacy'"
    ).fetchone()[0] == 0
    assert connection.execute("PRAGMA user_version").fetchone()[0] == 2
    connection.close()

    with app.app_context():
        db.session.remove()
        db.engine.dispose()

    # Restarting on the migrated database must not duplicate or rewrite data.
    app = create_app(Config, overrides)
    client = app.test_client()
    assert (
        client.post(
            "/api/auth/login",
            json={"username": "admin", "password": "admin123"},
        ).status_code
        == 200
    )
    assert client.get(
        "/api/workouts", query_string={"date": "2026-01-02"}
    ).get_json()["set_count"] == 1
    assert len(client.get("/api/admin/users").get_json()["users"]) == 2
    with app.app_context():
        db.session.remove()
        db.engine.dispose()

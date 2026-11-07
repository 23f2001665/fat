from __future__ import annotations

import secrets

from flask import current_app
from sqlalchemy import text
from werkzeug.security import generate_password_hash

from ..extensions import db
from .data_seed import seed_database
from .dates import now_timestamp
from .models import SchemaMigration, WorkoutLog


TRIGGERS = (
    """
    CREATE TRIGGER IF NOT EXISTS equipment_no_equipment_delete
    BEFORE DELETE ON equipment
    WHEN OLD.id = 1
    BEGIN
        SELECT RAISE(ABORT, 'No Equipment is reserved and cannot be deleted');
    END
    """,
    """
    CREATE TRIGGER IF NOT EXISTS equipment_no_equipment_deactivate
    BEFORE UPDATE ON equipment
    WHEN OLD.id = 1
         AND (NEW.id <> 1 OR NEW.is_active <> 1 OR NEW.name <> 'No Equipment')
    BEGIN
        SELECT RAISE(ABORT, 'No Equipment is reserved and must stay active');
    END
    """,
)

LEGACY_INDEXES = (
    "workout_logs_exercise_idx",
    "workout_logs_equipment_idx",
    "workout_logs_date_idx",
)


def _column_names(connection, table: str) -> set[str]:
    rows = connection.exec_driver_sql(f"PRAGMA table_info('{table}')")
    return {row[1] for row in rows}


def _table_names(connection) -> set[str]:
    rows = connection.execute(
        text("SELECT name FROM sqlite_master WHERE type = 'table'")
    )
    return {row[0] for row in rows}


def _admin_username() -> str:
    return str(current_app.config.get("ADMIN_USERNAME", "admin")).strip().lower()


def _admin_password() -> str:
    return str(current_app.config.get("ADMIN_PASSWORD", "admin123"))


def _migrate_legacy_users(connection) -> None:
    """Add authentication columns to databases created before RBAC existed."""

    if "users" not in _table_names(connection):
        return

    columns = _column_names(connection, "users")
    added_username = "username" not in columns

    additions = (
        ("username", "VARCHAR(64) NOT NULL DEFAULT ''"),
        ("password_hash", "VARCHAR(255) NOT NULL DEFAULT ''"),
        ("role", "VARCHAR(16) NOT NULL DEFAULT 'user'"),
        ("is_active", "BOOLEAN NOT NULL DEFAULT 1"),
    )
    for name, definition in additions:
        if name not in columns:
            connection.exec_driver_sql(
                f'ALTER TABLE users ADD COLUMN "{name}" {definition}'
            )

    rows = connection.execute(
        text("SELECT id, name, username, password_hash FROM users ORDER BY id")
    ).mappings().all()

    for row in rows:
        if not added_username and row["username"] and row["password_hash"]:
            continue
        is_primary = row["id"] == 1
        username = (
            _admin_username()
            if is_primary
            else (row["username"] or f"legacy{row['id']}")
        )
        password = _admin_password() if is_primary else secrets.token_urlsafe(32)
        role = "admin" if is_primary else "user"
        connection.execute(
            text(
                """
                UPDATE users
                SET username = :username,
                    password_hash = :password_hash,
                    role = :role,
                    is_active = 1
                WHERE id = :user_id
                """
            ),
            {
                "username": username,
                "password_hash": generate_password_hash(password),
                "role": role,
                "user_id": row["id"],
            },
        )

    connection.exec_driver_sql(
        "CREATE UNIQUE INDEX IF NOT EXISTS users_username_nocase "
        "ON users (lower(trim(username)))"
    )


def _legacy_owner_id(connection) -> int:
    """Find the account that should own pre-RBAC workout rows."""

    row = connection.execute(
        text(
            """
            SELECT id FROM users
            ORDER BY CASE WHEN role = 'admin' THEN 0 ELSE 1 END, id
            LIMIT 1
            """
        )
    ).first()
    if row is not None:
        return int(row[0])

    now = now_timestamp()
    connection.execute(
        text(
            """
            INSERT INTO users
                (id, name, username, password_hash, role, is_active,
                 created_at, updated_at)
            VALUES
                (1, 'Administrator', :username, :password_hash, 'admin', 1,
                 :created_at, :created_at)
            """
        ),
        {
            "username": _admin_username(),
            "password_hash": generate_password_hash(_admin_password()),
            "created_at": now,
        },
    )
    return 1


def _migrate_legacy_workout_logs(connection) -> None:
    """Rebuild workout_logs with a user_id primary-key component."""

    if "workout_logs" not in _table_names(connection):
        return
    if "user_id" in _column_names(connection, "workout_logs"):
        return

    owner_id = _legacy_owner_id(connection)

    for index_name in LEGACY_INDEXES:
        connection.exec_driver_sql(f'DROP INDEX IF EXISTS "{index_name}"')

    connection.exec_driver_sql(
        "ALTER TABLE workout_logs RENAME TO workout_logs_legacy"
    )
    WorkoutLog.__table__.create(bind=connection)
    connection.execute(
        text(
            """
            INSERT INTO workout_logs
                (user_id, date, sequence_number, exercise_id, equipment_id,
                 weight, complete_reps, partial_reps, rest_seconds, rir,
                 extra_info)
            SELECT :owner_id, date, sequence_number, exercise_id, equipment_id,
                   weight, complete_reps, partial_reps, rest_seconds, rir,
                   extra_info
            FROM workout_logs_legacy
            """
        ),
        {"owner_id": owner_id},
    )
    connection.exec_driver_sql("DROP TABLE workout_logs_legacy")


def _record_migration(version: int, description: str) -> None:
    if db.session.get(SchemaMigration, version) is None:
        db.session.add(
            SchemaMigration(
                version=version,
                description=description,
                applied_at=now_timestamp(),
            )
        )
        db.session.commit()


def bootstrap_database() -> None:
    db.create_all()

    with db.engine.begin() as connection:
        _migrate_legacy_users(connection)
        for statement in TRIGGERS:
            connection.execute(text(statement))

    # Seed users and the reference catalogue before copying legacy workout
    # rows, so the rebuilt workout_logs foreign keys have valid targets.
    seed_database()

    with db.engine.begin() as connection:
        _migrate_legacy_workout_logs(connection)
        connection.exec_driver_sql("PRAGMA user_version = 2")

    _record_migration(1, "Initial Flask/Vue workout tracker schema")
    _record_migration(
        2,
        "Role-based accounts, per-user workouts and session authentication",
    )

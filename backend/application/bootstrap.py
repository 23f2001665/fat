from __future__ import annotations

from sqlalchemy import text

from ..extensions import db
from .data_seed import seed_database
from .models import SchemaMigration
from .dates import now_timestamp


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


def bootstrap_database() -> None:
    db.create_all()

    with db.engine.begin() as connection:
        for statement in TRIGGERS:
            connection.execute(text(statement))
        connection.exec_driver_sql("PRAGMA user_version = 1")

    if db.session.get(SchemaMigration, 1) is None:
        db.session.add(
            SchemaMigration(
                version=1,
                description="Initial Flask/Vue workout tracker schema",
                applied_at=now_timestamp(),
            )
        )
        db.session.commit()

    seed_database()

from __future__ import annotations

import sqlite3

from flask_cors import CORS
from flask_sqlalchemy import SQLAlchemy
from sqlalchemy import event
from sqlalchemy.engine import Engine


db = SQLAlchemy()
cors = CORS()


@event.listens_for(Engine, "connect")
def _enable_sqlite_pragmas(dbapi_connection, _connection_record) -> None:
    """SQLite needs foreign keys enabled on every physical connection."""

    if isinstance(dbapi_connection, sqlite3.Connection):
        cursor = dbapi_connection.cursor()
        cursor.execute("PRAGMA foreign_keys = ON")
        cursor.execute("PRAGMA busy_timeout = 5000")
        cursor.close()

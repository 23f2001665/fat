from __future__ import annotations

import pytest

from backend.application import create_app
from backend.config import Config
from backend.extensions import db


@pytest.fixture()
def app(tmp_path):
    database = tmp_path / "test.db"
    application = create_app(
        Config,
        {
            "TESTING": True,
            "SQLALCHEMY_DATABASE_URI": f"sqlite:///{database}",
            "SEED_DATABASE": True,
            "CREATE_SCHEMA": True,
            "FRONTEND_DIST": tmp_path / "frontend-dist",
        },
    )
    yield application
    with application.app_context():
        db.session.remove()
        db.engine.dispose()


@pytest.fixture()
def client(app):
    return app.test_client()


@pytest.fixture()
def ctx(app):
    with app.app_context():
        yield app

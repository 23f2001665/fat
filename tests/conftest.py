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
            "SECRET_KEY": "test-secret",
            "ADMIN_USERNAME": "admin",
            "ADMIN_PASSWORD": "admin123",
            "DEFAULT_USERNAME": "athlete",
            "DEFAULT_PASSWORD": "athlete123",
        },
    )
    yield application
    with application.app_context():
        db.session.remove()
        db.engine.dispose()


def _signed_in(app, username: str, password: str):
    client = app.test_client()
    response = client.post(
        "/api/auth/login",
        json={"username": username, "password": password},
    )
    assert response.status_code == 200, response.get_json()
    return client


@pytest.fixture()
def anonymous_client(app):
    return app.test_client()


@pytest.fixture()
def admin_client(app):
    return _signed_in(app, "admin", "admin123")


@pytest.fixture()
def user_client(app):
    return _signed_in(app, "athlete", "athlete123")


@pytest.fixture()
def client(admin_client):
    """Most endpoint tests run as the administrator."""

    return admin_client


@pytest.fixture()
def ctx(app):
    with app.app_context():
        yield app

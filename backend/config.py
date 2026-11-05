from __future__ import annotations

import os

from pathlib import Path


BASE_DIR = Path(__file__).resolve().parent.parent


class Config:
    """Runtime configuration for the local-first tracker."""

    SECRET_KEY = os.getenv("SECRET_KEY", "fat-local-workout-tracker")
    SQLALCHEMY_DATABASE_URI = os.getenv(
        "DATABASE_URL",
        f"sqlite:///{BASE_DIR / 'instance' / 'workout.db'}",
    )
    SQLALCHEMY_TRACK_MODIFICATIONS = False
    SQLALCHEMY_ENGINE_OPTIONS = {"pool_pre_ping": True}
    JSON_SORT_KEYS = False
    FRONTEND_DIST = BASE_DIR / "frontend" / "dist"
    SEED_DATABASE = True
    CREATE_SCHEMA = True


class TestConfig(Config):
    TESTING = True
    SQLALCHEMY_DATABASE_URI = "sqlite:///:memory:"
    SECRET_KEY = "test"

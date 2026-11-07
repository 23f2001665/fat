from __future__ import annotations

from flask import Flask

from .analytics import bp as analytics_bp
from .auth import bp as auth_bp
from .library import bp as library_bp
from .meta import bp as meta_bp
from .profile import bp as profile_bp
from .users import bp as users_bp
from .workout import bp as workout_bp


BLUEPRINTS = (
    meta_bp,
    auth_bp,
    workout_bp,
    library_bp,
    profile_bp,
    analytics_bp,
    users_bp,
)


def register_blueprints(app: Flask) -> None:
    for blueprint in BLUEPRINTS:
        app.register_blueprint(blueprint, url_prefix="/api")

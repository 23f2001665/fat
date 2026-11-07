from __future__ import annotations

from functools import wraps

from flask import Flask, g, jsonify, request, session

from ..extensions import db
from .errors import ApiError
from .models import User


PUBLIC_API_PATHS = {"/api/health", "/api/auth/login", "/api/auth/logout"}


def init_auth(app: Flask) -> None:
    """Load the session user for every protected API request."""

    @app.before_request
    def load_session_user():
        if not request.path.startswith("/api"):
            return None
        if request.method == "OPTIONS" or request.path in PUBLIC_API_PATHS:
            return None

        user_id = session.get("user_id")
        user = db.session.get(User, user_id) if user_id is not None else None
        if user is None or not user.is_active:
            session.clear()
            if request.path == "/api/auth/me":
                return jsonify({"error": "Not signed in", "code": "authentication_required"}), 401
            return (
                jsonify(
                    {
                        "error": "Sign in to continue",
                        "code": "authentication_required",
                    }
                ),
                401,
            )
        g.current_user = user
        return None


def current_user() -> User:
    user = getattr(g, "current_user", None)
    if user is None:
        raise ApiError.unauthorized()
    return user


def login_required(view):
    @wraps(view)
    def wrapped(*args, **kwargs):
        current_user()
        return view(*args, **kwargs)

    return wrapped


def admin_required(view):
    @wraps(view)
    def wrapped(*args, **kwargs):
        if current_user().role != "admin":
            raise ApiError.forbidden("Only an administrator can change the library")
        return view(*args, **kwargs)

    return wrapped

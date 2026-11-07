from __future__ import annotations

import logging

from pathlib import Path

from flask import Flask, abort, jsonify, request, send_from_directory
from sqlalchemy.exc import IntegrityError, SQLAlchemyError
from werkzeug.exceptions import HTTPException

from ..config import Config
from ..extensions import db
from .auth import init_auth
from .bootstrap import bootstrap_database
from .errors import ApiError
from .routes import register_blueprints


def _prepare_sqlite_directory(uri: str) -> None:
    prefix = "sqlite:///"
    if not uri.startswith(prefix) or ":memory:" in uri:
        return
    path = Path(uri[len(prefix) :])
    path.parent.mkdir(parents=True, exist_ok=True)


def _register_error_handlers(app: Flask) -> None:
    @app.errorhandler(ApiError)
    def handle_api_error(error: ApiError):
        db.session.rollback()
        return (
            jsonify({"error": error.message, "code": error.code}),
            error.status_code,
        )

    @app.errorhandler(IntegrityError)
    def handle_integrity_error(error: IntegrityError):
        db.session.rollback()
        app.logger.warning("Database integrity error: %s", error)
        message = "The change conflicts with existing data."
        raw = str(getattr(error, "orig", error))
        if "No Equipment" in raw:
            message = "No Equipment is reserved and cannot be deleted or deactivated."
        return jsonify({"error": message, "code": "conflict"}), 409

    @app.errorhandler(404)
    def handle_not_found(_error):
        if request.path.startswith("/api"):
            return (
                jsonify({"error": "API endpoint not found", "code": "not_found"}),
                404,
            )
        return _serve_frontend(app)

    @app.errorhandler(405)
    def handle_method_not_allowed(_error):
        if request.path.startswith("/api"):
            return (
                jsonify({"error": "Method not allowed", "code": "method_not_allowed"}),
                405,
            )
        return _serve_frontend(app)

    @app.errorhandler(Exception)
    def handle_unexpected_error(error: Exception):
        if isinstance(error, HTTPException):
            return error
        db.session.rollback()
        app.logger.exception("Unhandled request error")
        if request.path.startswith("/api"):
            return (
                jsonify(
                    {
                        "error": "The server could not complete the request.",
                        "code": "internal_error",
                    }
                ),
                500,
            )
        return "Internal server error", 500


def _serve_frontend(app: Flask):
    dist = Path(app.config["FRONTEND_DIST"]).resolve()
    path = request.path.lstrip("/")
    if path and not path.startswith("api"):
        candidate = (dist / path).resolve()
        try:
            inside_dist = candidate == dist or dist in candidate.parents
        except ValueError:
            inside_dist = False
        if inside_dist and candidate.is_file():
            return send_from_directory(dist, path)

    index = dist / "index.html"
    if index.is_file():
        return send_from_directory(dist, "index.html")

    return (
        "<!doctype html><html><body style=\"font-family:system-ui;background:#0e1116;"
        "color:#eef2f7;padding:3rem;line-height:1.6\"><h1>FAT</h1>"
        "<p>The Vue frontend has not been built yet.</p>"
        "<p>Run <code>pnpm --dir frontend install &amp;&amp; "
        "pnpm --dir frontend build</code>, then reload this page.</p>"
        "</body></html>",
        503,
        {"Content-Type": "text/html; charset=utf-8"},
    )


def create_app(config_class=Config, overrides: dict | None = None) -> Flask:
    app = Flask(__name__, static_folder=None)
    app.config.from_object(config_class)
    if overrides:
        app.config.update(overrides)

    app.json.sort_keys = False
    _prepare_sqlite_directory(str(app.config["SQLALCHEMY_DATABASE_URI"]))

    db.init_app(app)
    init_auth(app)
    register_blueprints(app)
    _register_error_handlers(app)

    @app.get("/")
    @app.get("/<path:path>")
    def spa(path: str = ""):
        if path.startswith("api"):
            abort(404)
        return _serve_frontend(app)

    @app.after_request
    def api_cache_headers(response):
        if request.path.startswith("/api"):
            response.headers["Cache-Control"] = "no-store"
        return response

    if app.config.get("CREATE_SCHEMA", True):
        with app.app_context():
            bootstrap_database()

    if not app.logger.handlers:
        logging.basicConfig(level=logging.INFO)

    return app

from __future__ import annotations

from flask import Blueprint, jsonify


bp = Blueprint("meta", __name__)


@bp.get("/health")
def health():
    return jsonify(
        {
            "status": "ok",
            "service": "fat-workout-tracker",
            "version": "2.0.0",
        }
    )

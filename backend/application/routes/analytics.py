from __future__ import annotations

from flask import Blueprint, jsonify, request

from ..services import analytics_service
from .common import query_optional_date, query_optional_int


bp = Blueprint("analytics", __name__)


@bp.get("/analytics")
def get_analytics():
    return jsonify(
        analytics_service.build_report(
            period=request.args.get("period", "30"),
            start=query_optional_date("start", "Start date"),
            end=query_optional_date("end", "End date"),
            exercise_id=query_optional_int("exercise_id", "Exercise"),
            muscle_group_id=query_optional_int("muscle_group_id", "Muscle group"),
        )
    )

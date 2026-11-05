from __future__ import annotations

from flask import Blueprint, jsonify, request

from ..dates import DateRange, parse_optional_date, resolve_period
from ..services import profile_service
from .common import (
    json_object,
    payload_date,
    payload_optional_date,
    payload_optional_float,
    query_optional_date,
)


bp = Blueprint("profile", __name__)


@bp.get("/profile")
def get_profile():
    return jsonify(profile_service.profile_payload())


@bp.put("/profile")
def update_profile():
    data = json_object()
    user = profile_service.update_profile(
        data.get("name"),
        payload_optional_float(data, "height_cm", "Height"),
        payload_optional_date(data, "date_of_birth"),
    )
    from ..serializers import serialize_user

    return jsonify({"message": "Profile updated", "user": serialize_user(user)})


@bp.get("/profile/measurements")
def get_measurements():
    return jsonify(profile_service.measurement_payload())


@bp.post("/profile/measurements")
def add_measurement():
    data = json_object()
    measurement = profile_service.add_measurement(
        measured_at=payload_date(data, "measured_at"),
        weight_kg=payload_optional_float(data, "weight_kg", "Weight"),
        height_cm=payload_optional_float(data, "height_cm", "Height"),
        body_fat_percentage=payload_optional_float(
            data, "body_fat_percentage", "Body-fat percentage"
        ),
        waist_cm=payload_optional_float(data, "waist_cm", "Waist"),
        chest_cm=payload_optional_float(data, "chest_cm", "Chest"),
        hip_cm=payload_optional_float(data, "hip_cm", "Hip"),
        arm_cm=payload_optional_float(data, "arm_cm", "Arm"),
        thigh_cm=payload_optional_float(data, "thigh_cm", "Thigh"),
        notes=data.get("notes"),
    )
    from ..serializers import serialize_measurement

    return (
        jsonify(
            {
                "message": "Measurement added",
                "measurement": serialize_measurement(measurement),
            }
        ),
        201,
    )


@bp.get("/profile/series")
def metric_series():
    metric = request.args.get("metric", "weight")
    period = request.args.get("period", "all")
    start = query_optional_date("start", "Start date")
    end = query_optional_date("end", "End date")
    key, date_range, label = resolve_period(period, start, end)
    points = profile_service.metric_series(metric, date_range)
    return jsonify(
        {
            "metric": metric,
            "period": key,
            "period_label": label,
            "range": {
                "start": date_range.start.isoformat() if date_range.start else None,
                "end": date_range.end.isoformat() if date_range.end else None,
            },
            "points": points,
        }
    )

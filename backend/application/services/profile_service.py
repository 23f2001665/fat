from __future__ import annotations

import math

from datetime import date

from sqlalchemy import func

from ..calculations import bmi
from ..dates import DateRange, now_timestamp, today_local
from ..errors import ApiError
from ...extensions import db
from ..models import BodyMeasurement, User
from ..serializers import (
    metric_value,
    serialize_comparisons,
    serialize_latest_metrics,
    serialize_measurement,
    serialize_user,
)
from ..text import optional_text, required_name


MIN_DATE = date(1900, 1, 1)
MEASUREMENT_FIELDS = (
    "weight_kg",
    "height_cm",
    "body_fat_percentage",
    "waist_cm",
    "chest_cm",
    "hip_cm",
    "arm_cm",
    "thigh_cm",
)


def get_user(user_id: int) -> User:
    user = db.session.get(User, user_id)
    if user is None:
        raise ApiError.not_found("User profile is missing")
    return user


def update_profile(
    user_id: int,
    name: object,
    height_cm: float | None = None,
    date_of_birth: date | None = None,
) -> User:
    cleaned = required_name(name, "Name")
    if height_cm is not None:
        if not math.isfinite(float(height_cm)) or not 50 <= float(height_cm) <= 300:
            raise ApiError.validation("Height must be between 50 and 300 cm")
    if date_of_birth is not None:
        if date_of_birth > today_local():
            raise ApiError.validation("Date of birth cannot be in the future")
        if date_of_birth < MIN_DATE:
            raise ApiError.validation("Date of birth is too far in the past")
    user = get_user(user_id)
    user.name = cleaned
    user.height_cm = float(height_cm) if height_cm is not None else None
    user.date_of_birth = date_of_birth
    user.updated_at = now_timestamp()
    db.session.commit()
    return user


def _validate_measurement(
    measured_at: date,
    values: dict[str, float | None],
    notes: str | None,
) -> str | None:
    if measured_at > today_local():
        raise ApiError.validation("A measurement cannot be recorded for a future date")
    if measured_at < MIN_DATE:
        raise ApiError.validation("Measurement date is too far in the past")

    weight = values.get("weight_kg")
    height = values.get("height_cm")
    body_fat = values.get("body_fat_percentage")
    notes_value = optional_text(notes, "Notes")

    if not any(values.get(field) is not None for field in MEASUREMENT_FIELDS) and not notes_value:
        raise ApiError.validation("Enter at least one measurement or a note")
    if weight is not None:
        if not math.isfinite(float(weight)) or float(weight) <= 0 or float(weight) > 1000:
            raise ApiError.validation(
                "Weight must be greater than 0 and at most 1000 kg"
            )
    if height is not None:
        if not math.isfinite(float(height)) or float(height) <= 0:
            raise ApiError.validation("Height must be greater than zero")
        if float(height) > 300:
            raise ApiError.validation("Height must be at most 300 cm")
    if body_fat is not None:
        if not math.isfinite(float(body_fat)) or not 0 <= float(body_fat) <= 100:
            raise ApiError.validation("Body-fat percentage must be between 0 and 100")
    for field, label in (
        ("waist_cm", "Waist"),
        ("chest_cm", "Chest"),
        ("hip_cm", "Hip"),
        ("arm_cm", "Arm"),
        ("thigh_cm", "Thigh"),
    ):
        value = values.get(field)
        if value is not None:
            if not math.isfinite(float(value)) or not 0 <= float(value) <= 500:
                raise ApiError.validation(f"{label} must be between 0 and 500 cm")
    return notes_value


def add_measurement(
    user_id: int,
    measured_at: date,
    weight_kg: float | None = None,
    height_cm: float | None = None,
    body_fat_percentage: float | None = None,
    waist_cm: float | None = None,
    chest_cm: float | None = None,
    hip_cm: float | None = None,
    arm_cm: float | None = None,
    thigh_cm: float | None = None,
    notes: str | None = None,
) -> BodyMeasurement:
    values = {
        "weight_kg": weight_kg,
        "height_cm": height_cm,
        "body_fat_percentage": body_fat_percentage,
        "waist_cm": waist_cm,
        "chest_cm": chest_cm,
        "hip_cm": hip_cm,
        "arm_cm": arm_cm,
        "thigh_cm": thigh_cm,
    }
    notes_value = _validate_measurement(measured_at, values, notes)
    user = get_user(user_id)
    measurement = BodyMeasurement(
        user_id=user.id,
        measured_at=measured_at,
        weight_kg=float(weight_kg) if weight_kg is not None else None,
        height_cm=float(height_cm) if height_cm is not None else None,
        body_fat_percentage=(
            float(body_fat_percentage) if body_fat_percentage is not None else None
        ),
        waist_cm=float(waist_cm) if waist_cm is not None else None,
        chest_cm=float(chest_cm) if chest_cm is not None else None,
        hip_cm=float(hip_cm) if hip_cm is not None else None,
        arm_cm=float(arm_cm) if arm_cm is not None else None,
        thigh_cm=float(thigh_cm) if thigh_cm is not None else None,
        notes=notes_value,
    )
    db.session.add(measurement)
    db.session.commit()
    return measurement


def measurement_history(user_id: int) -> list[BodyMeasurement]:
    return (
        db.session.query(BodyMeasurement)
        .filter(BodyMeasurement.user_id == user_id)
        .order_by(BodyMeasurement.measured_at.desc(), BodyMeasurement.id.desc())
        .all()
    )


def latest_measurement(user_id: int) -> BodyMeasurement | None:
    history = measurement_history(user_id)
    return history[0] if history else None


def latest_metrics(user_id: int) -> dict:
    return serialize_latest_metrics(measurement_history(user_id))


def comparisons(user_id: int) -> list[dict]:
    return serialize_comparisons(measurement_history(user_id))


def metric_series(user_id: int, metric: str, date_range: DateRange) -> list[dict]:
    if metric not in {"weight", "bmi", "body-fat", "waist", "chest", "arm", "thigh"}:
        raise ApiError.validation(
            "Metric must be weight, bmi, body-fat, waist, chest, arm or thigh"
        )
    query = db.session.query(BodyMeasurement).filter(
        BodyMeasurement.user_id == user_id
    )
    if date_range.start is not None:
        query = query.filter(BodyMeasurement.measured_at >= date_range.start)
    if date_range.end is not None:
        query = query.filter(BodyMeasurement.measured_at <= date_range.end)
    measurements = query.order_by(
        BodyMeasurement.measured_at.asc(), BodyMeasurement.id.asc()
    ).all()
    return [
        {
            "date": measurement.measured_at.isoformat(),
            "value": value,
        }
        for measurement in measurements
        if (value := metric_value(measurement, metric)) is not None
    ]


def profile_payload(user_id: int) -> dict:
    user = get_user(user_id)
    history = measurement_history(user_id)
    return {
        "user": serialize_user(user),
        "latest_metrics": serialize_latest_metrics(history),
        "comparisons": serialize_comparisons(history),
        "measurement_count": len(history),
        "measurements": [serialize_measurement(item) for item in history],
    }


def measurement_payload(user_id: int) -> dict:
    history = measurement_history(user_id)
    return {
        "measurements": [serialize_measurement(item) for item in history],
        "latest_metrics": serialize_latest_metrics(history),
        "comparisons": serialize_comparisons(history),
        "measurement_count": len(history),
    }


def bmi_from_snapshot(weight_kg: float | None, height_cm: float | None) -> float | None:
    return bmi(weight_kg, height_cm)


def measurement_count(user_id: int) -> int:
    return (
        db.session.query(func.count(BodyMeasurement.id))
        .filter(BodyMeasurement.user_id == user_id)
        .scalar()
        or 0
    )

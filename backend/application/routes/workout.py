from __future__ import annotations

from flask import Blueprint, jsonify, request

from ..auth import current_user
from ..dates import parse_date, today_local
from ..serializers import serialize_day, serialize_set
from ..services import workout_service
from .common import (
    json_object,
    payload_date,
    payload_float,
    payload_int,
    payload_optional_int,
)


bp = Blueprint("workouts", __name__)


@bp.get("/workouts")
def workout_day():
    raw_date = request.args.get("date")
    workout_date = parse_date(raw_date, "Date") if raw_date else today_local()
    sets = workout_service.workout_for_date(current_user().id, workout_date)
    return jsonify(serialize_day(workout_date, sets))


@bp.get("/workouts/history")
def workout_history():
    return jsonify({"days": workout_service.history(current_user().id)})


@bp.get("/workouts/<date_value>/<int:sequence_number>")
def workout_set(date_value: str, sequence_number: int):
    workout_date = parse_date(date_value, "Date")
    return jsonify(
        {
            "set": serialize_set(
                workout_service.get_set(
                    current_user().id, workout_date, sequence_number
                )
            )
        }
    )


@bp.post("/workouts")
def add_workout_set():
    data = json_object()
    workout_set = workout_service.add_set(
        user_id=current_user().id,
        workout_date=payload_date(data),
        exercise_id=payload_int(data, "exercise_id", "Exercise"),
        equipment_id=payload_int(data, "equipment_id", "Equipment"),
        weight=payload_float(data, "weight", "Weight"),
        complete_reps=payload_optional_int(data, "complete_reps", "Complete reps") or 0,
        partial_reps=payload_optional_int(data, "partial_reps", "Partial reps") or 0,
        rest_seconds=payload_optional_int(data, "rest_seconds", "Rest duration"),
        rir=payload_optional_int(data, "rir", "RIR"),
        extra_info=data.get("extra_info"),
    )
    return (
        jsonify(
            {
                "message": f"Set {workout_set.sequence_number} logged",
                "set": serialize_set(workout_set),
            }
        ),
        201,
    )


@bp.put("/workouts/<date_value>/<int:sequence_number>")
def update_workout_set(date_value: str, sequence_number: int):
    data = json_object()
    workout_set = workout_service.update_set(
        user_id=current_user().id,
        workout_date=parse_date(date_value, "Date"),
        sequence_number=sequence_number,
        exercise_id=payload_int(data, "exercise_id", "Exercise"),
        equipment_id=payload_int(data, "equipment_id", "Equipment"),
        weight=payload_float(data, "weight", "Weight"),
        complete_reps=payload_optional_int(data, "complete_reps", "Complete reps") or 0,
        partial_reps=payload_optional_int(data, "partial_reps", "Partial reps") or 0,
        rest_seconds=payload_optional_int(data, "rest_seconds", "Rest duration"),
        rir=payload_optional_int(data, "rir", "RIR"),
        extra_info=data.get("extra_info"),
    )
    return jsonify({"message": "Set updated", "set": serialize_set(workout_set)})


@bp.delete("/workouts/<date_value>/<int:sequence_number>")
def delete_workout_set(date_value: str, sequence_number: int):
    workout_service.delete_set(
        current_user().id, parse_date(date_value, "Date"), sequence_number
    )
    return jsonify({"message": "Set deleted"})

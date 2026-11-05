from __future__ import annotations

import math
import threading

from datetime import date, timedelta

from sqlalchemy import func

from ..calculations import effective_reps
from ..dates import format_date, today_local
from ..errors import ApiError
from ...extensions import db
from ..models import Equipment, Exercise, WorkoutLog
from ..text import optional_text


_sequence_lock = threading.Lock()
MIN_WORKOUT_DATE = date(1900, 1, 1)


def validate_workout_values(
    weight: float,
    complete_reps: int,
    partial_reps: int,
    rest_seconds: int | None,
    rir: int | None,
) -> None:
    if isinstance(weight, bool) or not isinstance(weight, (int, float)):
        raise ApiError.validation("Weight must be a number")
    if not math.isfinite(float(weight)) or float(weight) < 0:
        raise ApiError.validation("Weight must be a finite, non-negative number")
    if float(weight) > 2000:
        raise ApiError.validation("Weight looks unreasonable (max 2000 kg)")
    if not isinstance(complete_reps, int) or isinstance(complete_reps, bool):
        raise ApiError.validation("Complete reps must be a whole number")
    if not isinstance(partial_reps, int) or isinstance(partial_reps, bool):
        raise ApiError.validation("Partial reps must be a whole number")
    if not 0 <= complete_reps <= 10000:
        raise ApiError.validation("Complete reps must be between 0 and 10000")
    if not 0 <= partial_reps <= 10000:
        raise ApiError.validation("Partial reps must be between 0 and 10000")
    if complete_reps == 0 and partial_reps == 0:
        raise ApiError.validation("A set must have at least one complete or partial rep")
    if rest_seconds is not None and not 0 <= rest_seconds <= 86400:
        raise ApiError.validation("Rest duration must be between 0 and 86400 seconds")
    if rir is not None and not 0 <= rir <= 10:
        raise ApiError.validation("RIR must be between 0 and 10")


def validate_workout_date(workout_date: date) -> None:
    if workout_date > today_local():
        raise ApiError.validation("Workouts cannot be logged for a future date")
    if workout_date < MIN_WORKOUT_DATE:
        raise ApiError.validation("Workout date is too far in the past")


def _get_exercise(exercise_id: int, require_active: bool) -> Exercise:
    exercise = db.session.get(Exercise, exercise_id)
    if exercise is None:
        raise ApiError.not_found("Exercise not found")
    if require_active and not exercise.is_active:
        raise ApiError.validation(
            f"'{exercise.name}' is inactive and cannot be used for a new set"
        )
    return exercise


def _get_equipment(equipment_id: int, require_active: bool) -> Equipment:
    equipment = db.session.get(Equipment, equipment_id)
    if equipment is None:
        raise ApiError.not_found("Equipment not found")
    if require_active and not equipment.is_active:
        raise ApiError.validation(
            f"'{equipment.name}' is inactive and cannot be used for a new set"
        )
    return equipment


def add_set(
    workout_date: date,
    exercise_id: int,
    equipment_id: int,
    weight: float,
    complete_reps: int,
    partial_reps: int,
    rest_seconds: int | None = None,
    rir: int | None = None,
    extra_info: str | None = None,
) -> WorkoutLog:
    validate_workout_date(workout_date)
    validate_workout_values(weight, complete_reps, partial_reps, rest_seconds, rir)
    _get_exercise(exercise_id, True)
    _get_equipment(equipment_id, True)
    info = optional_text(extra_info, "Extra information")

    # The lock and unique (date, sequence_number) key keep sequence allocation
    # deterministic even if two requests arrive at the same moment.
    with _sequence_lock:
        next_sequence = (
            db.session.query(func.coalesce(func.max(WorkoutLog.sequence_number), 0))
            .filter(WorkoutLog.date == workout_date)
            .scalar()
            or 0
        ) + 1
        workout_set = WorkoutLog(
            date=workout_date,
            sequence_number=next_sequence,
            exercise_id=exercise_id,
            equipment_id=equipment_id,
            weight=float(weight),
            complete_reps=complete_reps,
            partial_reps=partial_reps,
            rest_seconds=rest_seconds,
            rir=rir,
            extra_info=info,
        )
        db.session.add(workout_set)
        db.session.commit()
    return workout_set


def get_set(workout_date: date, sequence_number: int) -> WorkoutLog:
    workout_set = db.session.get(WorkoutLog, (workout_date, sequence_number))
    if workout_set is None:
        raise ApiError.not_found("Workout set not found")
    return workout_set


def update_set(
    workout_date: date,
    sequence_number: int,
    exercise_id: int,
    equipment_id: int,
    weight: float,
    complete_reps: int,
    partial_reps: int,
    rest_seconds: int | None = None,
    rir: int | None = None,
    extra_info: str | None = None,
) -> WorkoutLog:
    validate_workout_date(workout_date)
    validate_workout_values(weight, complete_reps, partial_reps, rest_seconds, rir)
    workout_set = get_set(workout_date, sequence_number)
    existing_exercise_id = workout_set.exercise_id
    existing_equipment_id = workout_set.equipment_id

    # Deactivated records stay valid for the set that already references them.
    _get_exercise(exercise_id, exercise_id != existing_exercise_id)
    _get_equipment(equipment_id, equipment_id != existing_equipment_id)
    info = optional_text(extra_info, "Extra information")

    workout_set.exercise_id = exercise_id
    workout_set.equipment_id = equipment_id
    workout_set.weight = float(weight)
    workout_set.complete_reps = complete_reps
    workout_set.partial_reps = partial_reps
    workout_set.rest_seconds = rest_seconds
    workout_set.rir = rir
    workout_set.extra_info = info
    db.session.commit()
    return workout_set


def delete_set(workout_date: date, sequence_number: int) -> None:
    workout_set = get_set(workout_date, sequence_number)
    db.session.delete(workout_set)
    db.session.commit()


def workout_for_date(workout_date: date) -> list[WorkoutLog]:
    return (
        db.session.query(WorkoutLog)
        .filter(WorkoutLog.date == workout_date)
        .order_by(WorkoutLog.sequence_number.asc())
        .all()
    )


def today_workout() -> list[WorkoutLog]:
    return workout_for_date(today_local())


def next_sequence(workout_date: date) -> int:
    current = (
        db.session.query(func.max(WorkoutLog.sequence_number))
        .filter(WorkoutLog.date == workout_date)
        .scalar()
    )
    return (current or 0) + 1


def history() -> list[dict]:
    totals = db.session.query(
        WorkoutLog.date.label("date"),
        func.count().label("set_count"),
        func.sum(
            WorkoutLog.weight
            * (WorkoutLog.complete_reps + WorkoutLog.partial_reps / 2.0)
        ).label("total_volume"),
    ).group_by(WorkoutLog.date).order_by(WorkoutLog.date.desc()).all()

    names = db.session.query(
        WorkoutLog.date,
        Exercise.name,
        func.min(WorkoutLog.sequence_number).label("first_sequence"),
    ).join(Exercise, Exercise.id == WorkoutLog.exercise_id).group_by(
        WorkoutLog.date, Exercise.id, Exercise.name
    ).order_by(
        WorkoutLog.date.desc(), func.min(WorkoutLog.sequence_number).asc()
    ).all()

    names_by_date: dict[date, list[str]] = {}
    for workout_date, exercise_name, _ in names:
        names_by_date.setdefault(workout_date, []).append(exercise_name)

    return [
        {
            "date": format_date(row.date),
            "set_count": row.set_count,
            "total_volume": float(row.total_volume or 0.0),
            "exercise_names": names_by_date.get(row.date, []),
        }
        for row in totals
    ]


def date_label(value: date) -> str:
    return format_date(value)


def exercises_performed(workout_date: date) -> list[str]:
    names: list[str] = []
    for workout_set in workout_for_date(workout_date):
        if workout_set.exercise.name not in names:
            names.append(workout_set.exercise.name)
    return names


def preview_day(workout_date: date) -> dict:
    sets = workout_for_date(workout_date)
    return {
        "date": format_date(workout_date),
        "sets": sets,
        "next_sequence": next_sequence(workout_date),
        "date_min": (today_local() - timedelta(days=36500)).isoformat(),
    }


def effective_reps_for(workout_set: WorkoutLog) -> float:
    return effective_reps(workout_set.complete_reps, workout_set.partial_reps)

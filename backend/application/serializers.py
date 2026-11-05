from __future__ import annotations

from .calculations import bmi_category
from .dates import format_date
from .models import BodyMeasurement, Equipment, Exercise, MuscleGroup, User, WorkoutLog


METRIC_DEFINITIONS: list[tuple[str, str, str]] = [
    ("weight", "Body weight", "kg"),
    ("bmi", "BMI", ""),
    ("body-fat", "Body-fat %", "%"),
    ("waist", "Waist", "cm"),
    ("chest", "Chest", "cm"),
    ("arm", "Arm", "cm"),
    ("thigh", "Thigh", "cm"),
]


def serialize_user(user: User) -> dict:
    return {
        "id": user.id,
        "name": user.name,
        "height_cm": user.height_cm,
        "date_of_birth": format_date(user.date_of_birth) if user.date_of_birth else None,
        "created_at": user.created_at,
        "updated_at": user.updated_at,
    }


def serialize_muscle_group(muscle: MuscleGroup) -> dict:
    return {
        "id": muscle.id,
        "name": muscle.name,
        "description": muscle.description,
        "is_active": bool(muscle.is_active),
    }


def serialize_equipment(equipment: Equipment) -> dict:
    return {
        "id": equipment.id,
        "name": equipment.name,
        "category": equipment.category,
        "is_active": bool(equipment.is_active),
        "is_reserved": equipment.id == 1,
    }


def serialize_exercise(exercise: Exercise) -> dict:
    muscles = sorted(
        exercise.muscle_links,
        key=lambda link: (0 if link.role == "Primary" else 1, link.muscle_group.name.lower()),
    )
    rows = [
        {
            "muscle_group_id": link.muscle_group_id,
            "muscle_group_name": link.muscle_group.name,
            "role": link.role,
        }
        for link in muscles
    ]
    summary = (
        ", ".join(
            f"{row['muscle_group_name']} ({'P' if row['role'] == 'Primary' else 'S'})"
            for row in rows
        )
        if rows
        else "No muscle groups assigned"
    )
    return {
        "id": exercise.id,
        "name": exercise.name,
        "normalized_name": exercise.normalized_name,
        "description": exercise.description,
        "category": exercise.category,
        "is_active": bool(exercise.is_active),
        "muscles": rows,
        "muscle_summary": summary,
    }


def serialize_set(workout_set: WorkoutLog) -> dict:
    return {
        "date": format_date(workout_set.date),
        "sequence_number": workout_set.sequence_number,
        "exercise_id": workout_set.exercise_id,
        "exercise_name": workout_set.exercise.name,
        "equipment_id": workout_set.equipment_id,
        "equipment_name": workout_set.equipment.name,
        "weight": workout_set.weight,
        "complete_reps": workout_set.complete_reps,
        "partial_reps": workout_set.partial_reps,
        "effective_reps": workout_set.effective_reps,
        "volume": workout_set.volume,
        "rest_seconds": workout_set.rest_seconds,
        "rir": workout_set.rir,
        "extra_info": workout_set.extra_info,
    }


def serialize_day(workout_date, sets: list[WorkoutLog]) -> dict:
    names: list[str] = []
    for workout_set in sets:
        if workout_set.exercise.name not in names:
            names.append(workout_set.exercise.name)
    return {
        "date": format_date(workout_date),
        "sets": [serialize_set(workout_set) for workout_set in sets],
        "total_volume": sum(workout_set.volume for workout_set in sets),
        "exercise_names": names,
        "set_count": len(sets),
    }


def serialize_measurement(measurement: BodyMeasurement) -> dict:
    bmi_value = measurement.bmi
    return {
        "id": measurement.id,
        "user_id": measurement.user_id,
        "measured_at": format_date(measurement.measured_at),
        "weight_kg": measurement.weight_kg,
        "height_cm": measurement.height_cm,
        "body_fat_percentage": measurement.body_fat_percentage,
        "waist_cm": measurement.waist_cm,
        "chest_cm": measurement.chest_cm,
        "hip_cm": measurement.hip_cm,
        "arm_cm": measurement.arm_cm,
        "thigh_cm": measurement.thigh_cm,
        "notes": measurement.notes,
        "bmi": bmi_value,
        "bmi_category": bmi_category(bmi_value),
    }


def metric_value(measurement: BodyMeasurement, key: str) -> float | None:
    return {
        "weight": measurement.weight_kg,
        "bmi": measurement.bmi,
        "body-fat": measurement.body_fat_percentage,
        "waist": measurement.waist_cm,
        "chest": measurement.chest_cm,
        "arm": measurement.arm_cm,
        "thigh": measurement.thigh_cm,
    }.get(key)


def serialize_comparisons(history: list[BodyMeasurement]) -> list[dict]:
    """Latest two observations for each metric, independently."""

    comparisons: list[dict] = []
    for key, label, unit in METRIC_DEFINITIONS:
        observations: list[tuple[float, str]] = []
        for measurement in history:
            value = metric_value(measurement, key)
            if value is not None:
                observations.append((value, format_date(measurement.measured_at)))
        current_value, current_date = observations[0] if observations else (None, None)
        previous_value, previous_date = observations[1] if len(observations) > 1 else (None, None)
        delta = (
            current_value - previous_value
            if current_value is not None and previous_value is not None
            else None
        )
        comparisons.append(
            {
                "metric": key,
                "label": label,
                "unit": unit,
                "current": current_value,
                "current_date": current_date,
                "previous": previous_value,
                "previous_date": previous_date,
                "delta": delta,
            }
        )
    return comparisons


def serialize_latest_metrics(history: list[BodyMeasurement]) -> dict:
    metrics: dict = {
        "latest_measurement_date": format_date(history[0].measured_at) if history else None,
        "weight_kg": None,
        "height_cm": None,
        "body_fat_percentage": None,
        "waist_cm": None,
        "chest_cm": None,
        "hip_cm": None,
        "arm_cm": None,
        "thigh_cm": None,
        "bmi": None,
        "bmi_category": "Unavailable",
        "bmi_measured_at": None,
    }

    field_map = {
        "weight_kg": "weight_kg",
        "height_cm": "height_cm",
        "body_fat_percentage": "body_fat_percentage",
        "waist_cm": "waist_cm",
        "chest_cm": "chest_cm",
        "hip_cm": "hip_cm",
        "arm_cm": "arm_cm",
        "thigh_cm": "thigh_cm",
    }
    for measurement in history:
        for metric_key, attr in field_map.items():
            if metrics[metric_key] is None:
                metrics[metric_key] = getattr(measurement, attr)
        if metrics["bmi"] is None and measurement.bmi is not None:
            metrics["bmi"] = measurement.bmi
            metrics["bmi_measured_at"] = format_date(measurement.measured_at)

    metrics["bmi_category"] = bmi_category(metrics["bmi"])
    return metrics

from __future__ import annotations

from flask import Blueprint, jsonify, request

from ..serializers import (
    serialize_equipment,
    serialize_exercise,
    serialize_muscle_group,
)
from ..services import library_service
from .common import json_object, query_bool


bp = Blueprint("library", __name__)


def _search() -> str | None:
    return request.args.get("search")


@bp.get("/library/exercises")
def list_exercises():
    rows = library_service.list_exercises(query_bool("include_inactive"), _search())
    return jsonify({"exercises": [serialize_exercise(row) for row in rows]})


@bp.get("/library/exercises/<int:exercise_id>")
def get_exercise(exercise_id: int):
    return jsonify({"exercise": serialize_exercise(library_service.get_exercise(exercise_id))})


@bp.post("/library/exercises")
def create_exercise():
    data = json_object()
    exercise = library_service.create_exercise(
        data.get("name"),
        data.get("description"),
        data.get("category", "Other"),
        data.get("muscles", []),
    )
    return jsonify({"message": "Exercise created", "exercise": serialize_exercise(exercise)}), 201


@bp.put("/library/exercises/<int:exercise_id>")
def update_exercise(exercise_id: int):
    data = json_object()
    exercise = library_service.update_exercise(
        exercise_id,
        data.get("name"),
        data.get("description"),
        data.get("category", "Other"),
        data.get("is_active", True),
        data.get("muscles", []),
    )
    return jsonify({"message": "Exercise updated", "exercise": serialize_exercise(exercise)})


@bp.patch("/library/exercises/<int:exercise_id>/active")
def set_exercise_active(exercise_id: int):
    data = json_object()
    exercise = library_service.set_exercise_active(exercise_id, data.get("is_active"))
    return jsonify({"message": "Exercise updated", "exercise": serialize_exercise(exercise)})


@bp.get("/library/muscle-groups")
def list_muscle_groups():
    rows = library_service.list_muscle_groups(query_bool("include_inactive"), _search())
    return jsonify({"muscle_groups": [serialize_muscle_group(row) for row in rows]})


@bp.get("/library/muscle-groups/<int:muscle_group_id>")
def get_muscle_group(muscle_group_id: int):
    return jsonify(
        {"muscle_group": serialize_muscle_group(library_service.get_muscle_group(muscle_group_id))}
    )


@bp.post("/library/muscle-groups")
def create_muscle_group():
    data = json_object()
    muscle = library_service.create_muscle_group(data.get("name"), data.get("description"))
    return jsonify({"message": "Muscle group created", "muscle_group": serialize_muscle_group(muscle)}), 201


@bp.put("/library/muscle-groups/<int:muscle_group_id>")
def update_muscle_group(muscle_group_id: int):
    data = json_object()
    muscle = library_service.update_muscle_group(
        muscle_group_id,
        data.get("name"),
        data.get("description"),
        data.get("is_active", True),
    )
    return jsonify({"message": "Muscle group updated", "muscle_group": serialize_muscle_group(muscle)})


@bp.patch("/library/muscle-groups/<int:muscle_group_id>/active")
def set_muscle_group_active(muscle_group_id: int):
    data = json_object()
    muscle = library_service.set_muscle_group_active(muscle_group_id, data.get("is_active"))
    return jsonify({"message": "Muscle group updated", "muscle_group": serialize_muscle_group(muscle)})


@bp.get("/library/equipment")
def list_equipment():
    rows = library_service.list_equipment(query_bool("include_inactive"), _search())
    return jsonify({"equipment": [serialize_equipment(row) for row in rows]})


@bp.get("/library/equipment/<int:equipment_id>")
def get_equipment(equipment_id: int):
    return jsonify({"equipment": serialize_equipment(library_service.get_equipment(equipment_id))})


@bp.post("/library/equipment")
def create_equipment():
    data = json_object()
    equipment = library_service.create_equipment(data.get("name"), data.get("category"))
    return jsonify({"message": "Equipment created", "equipment": serialize_equipment(equipment)}), 201


@bp.put("/library/equipment/<int:equipment_id>")
def update_equipment(equipment_id: int):
    data = json_object()
    equipment = library_service.update_equipment(
        equipment_id,
        data.get("name"),
        data.get("category"),
        data.get("is_active", True),
    )
    return jsonify({"message": "Equipment updated", "equipment": serialize_equipment(equipment)})


@bp.patch("/library/equipment/<int:equipment_id>/active")
def set_equipment_active(equipment_id: int):
    data = json_object()
    equipment = library_service.set_equipment_active(equipment_id, data.get("is_active"))
    return jsonify({"message": "Equipment updated", "equipment": serialize_equipment(equipment)})

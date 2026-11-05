from __future__ import annotations

from sqlalchemy import case, func, or_

from ..errors import ApiError
from ...extensions import db
from ..models import Exercise, ExerciseMuscle, Equipment, MuscleGroup
from ..text import (
    clean_name,
    normalize_name,
    optional_text,
    required_name,
)


CATEGORIES = ("Compound", "Isolation", "Bodyweight", "Other")
ROLES = ("Primary", "Secondary")
NO_EQUIPMENT_ID = 1


def list_exercises(include_inactive: bool, search: str | None = None) -> list[Exercise]:
    query = db.session.query(Exercise)
    if not include_inactive:
        query = query.filter(Exercise.is_active.is_(True))
    needle = (search or "").strip().lower()
    if needle:
        pattern = f"%{needle}%"
        query = query.filter(
            or_(
                func.lower(Exercise.name).like(pattern),
                func.lower(func.coalesce(Exercise.description, "")).like(pattern),
            )
        )
    return query.order_by(Exercise.is_active.desc(), func.lower(Exercise.name).asc()).all()


def get_exercise(exercise_id: int) -> Exercise:
    exercise = db.session.get(Exercise, exercise_id)
    if exercise is None:
        raise ApiError.not_found("Exercise not found")
    return exercise


def _validate_category(category: object) -> str:
    if category is None or (isinstance(category, str) and not category.strip()):
        return "Other"
    if not isinstance(category, str) or category not in CATEGORIES:
        raise ApiError.validation(
            "Category must be one of Compound, Isolation, Bodyweight or Other"
        )
    return category


def _validate_assignments(assignments: object) -> list[tuple[int, str]]:
    if assignments is None:
        return []
    if not isinstance(assignments, list):
        raise ApiError.validation("Muscles must be a list")
    seen: set[int] = set()
    validated: list[tuple[int, str]] = []
    for assignment in assignments:
        if not isinstance(assignment, dict):
            raise ApiError.validation("Each muscle assignment must be an object")
        muscle_id = assignment.get("muscle_group_id")
        if isinstance(muscle_id, bool) or not isinstance(muscle_id, int):
            raise ApiError.validation("Muscle group is required for each assignment")
        if muscle_id in seen:
            raise ApiError.validation("A muscle group can only be assigned once per exercise")
        seen.add(muscle_id)
        role = assignment.get("role")
        if role not in ROLES:
            raise ApiError.validation("Muscle role must be Primary or Secondary")
        muscle = db.session.get(MuscleGroup, muscle_id)
        if muscle is None:
            raise ApiError.not_found("Muscle group not found")
        if not muscle.is_active:
            raise ApiError.validation(f"'{muscle.name}' is inactive and cannot be assigned")
        validated.append((muscle_id, role))
    return validated


def _duplicate_exercise(normalized: str) -> Exercise | None:
    return (
        db.session.query(Exercise)
        .filter(Exercise.normalized_name == normalized)
        .first()
    )


def create_exercise(
    name: object,
    description: object = None,
    category: object = "Other",
    muscles: object = None,
) -> Exercise:
    cleaned = required_name(name, "Exercise name")
    normalized = normalize_name(cleaned)
    existing = _duplicate_exercise(normalized)
    if existing is not None:
        raise ApiError.conflict(f"An exercise named '{existing.name}' already exists")
    validated_muscles = _validate_assignments(muscles)
    exercise = Exercise(
        name=cleaned,
        normalized_name=normalized,
        description=optional_text(description, "Description"),
        category=_validate_category(category),
        is_active=True,
    )
    db.session.add(exercise)
    db.session.flush()
    for muscle_id, role in validated_muscles:
        db.session.add(
            ExerciseMuscle(
                exercise_id=exercise.id,
                muscle_group_id=muscle_id,
                role=role,
            )
        )
    db.session.commit()
    return exercise


def update_exercise(
    exercise_id: int,
    name: object,
    description: object,
    category: object,
    is_active: bool,
    muscles: object,
) -> Exercise:
    exercise = get_exercise(exercise_id)
    cleaned = required_name(name, "Exercise name")
    normalized = normalize_name(cleaned)
    existing = _duplicate_exercise(normalized)
    if existing is not None and existing.id != exercise_id:
        raise ApiError.conflict(f"An exercise named '{existing.name}' already exists")
    if not isinstance(is_active, bool):
        raise ApiError.validation("Active must be true or false")
    validated_muscles = _validate_assignments(muscles)

    exercise.name = cleaned
    exercise.normalized_name = normalized
    exercise.description = optional_text(description, "Description")
    exercise.category = _validate_category(category)
    exercise.is_active = is_active

    for link in list(exercise.muscle_links):
        db.session.delete(link)
    db.session.flush()
    for muscle_id, role in validated_muscles:
        db.session.add(
            ExerciseMuscle(
                exercise_id=exercise.id,
                muscle_group_id=muscle_id,
                role=role,
            )
        )
    db.session.commit()
    return exercise


def set_exercise_active(exercise_id: int, is_active: bool) -> Exercise:
    if not isinstance(is_active, bool):
        raise ApiError.validation("Active must be true or false")
    exercise = get_exercise(exercise_id)
    exercise.is_active = is_active
    db.session.commit()
    return exercise


def list_muscle_groups(include_inactive: bool, search: str | None = None) -> list[MuscleGroup]:
    query = db.session.query(MuscleGroup)
    if not include_inactive:
        query = query.filter(MuscleGroup.is_active.is_(True))
    needle = (search or "").strip().lower()
    if needle:
        pattern = f"%{needle}%"
        query = query.filter(
            or_(
                func.lower(MuscleGroup.name).like(pattern),
                func.lower(func.coalesce(MuscleGroup.description, "")).like(pattern),
            )
        )
    return query.order_by(
        MuscleGroup.is_active.desc(), func.lower(MuscleGroup.name).asc()
    ).all()


def get_muscle_group(muscle_group_id: int) -> MuscleGroup:
    muscle = db.session.get(MuscleGroup, muscle_group_id)
    if muscle is None:
        raise ApiError.not_found("Muscle group not found")
    return muscle


def muscle_group_name_exists(name: str, excluding_id: int | None = None) -> bool:
    query = db.session.query(MuscleGroup).filter(
        func.lower(func.trim(MuscleGroup.name)) == clean_name(name).lower()
    )
    if excluding_id is not None:
        query = query.filter(MuscleGroup.id != excluding_id)
    return query.first() is not None


def create_muscle_group(name: object, description: object = None) -> MuscleGroup:
    cleaned = required_name(name, "Muscle group name")
    if muscle_group_name_exists(cleaned):
        raise ApiError.conflict("A muscle group with that name already exists")
    muscle = MuscleGroup(
        name=cleaned,
        description=optional_text(description, "Description"),
        is_active=True,
    )
    db.session.add(muscle)
    db.session.commit()
    return muscle


def update_muscle_group(
    muscle_group_id: int,
    name: object,
    description: object,
    is_active: bool,
) -> MuscleGroup:
    muscle = get_muscle_group(muscle_group_id)
    cleaned = required_name(name, "Muscle group name")
    if muscle_group_name_exists(cleaned, excluding_id=muscle_group_id):
        raise ApiError.conflict("A muscle group with that name already exists")
    if not isinstance(is_active, bool):
        raise ApiError.validation("Active must be true or false")
    if muscle.is_active and not is_active:
        association_count = (
            db.session.query(func.count())
            .select_from(ExerciseMuscle)
            .filter(ExerciseMuscle.muscle_group_id == muscle_group_id)
            .scalar()
        )
        if association_count:
            raise ApiError.validation(
                "This muscle group is used by exercises and cannot be deactivated. "
                "Remove it from those exercise definitions first."
            )
    muscle.name = cleaned
    muscle.description = optional_text(description, "Description")
    muscle.is_active = is_active
    db.session.commit()
    return muscle


def set_muscle_group_active(muscle_group_id: int, is_active: bool) -> MuscleGroup:
    muscle = get_muscle_group(muscle_group_id)
    return update_muscle_group(
        muscle_group_id, muscle.name, muscle.description, is_active
    )


def list_equipment(include_inactive: bool, search: str | None = None) -> list[Equipment]:
    query = db.session.query(Equipment)
    if not include_inactive:
        query = query.filter(Equipment.is_active.is_(True))
    needle = (search or "").strip().lower()
    if needle:
        pattern = f"%{needle}%"
        query = query.filter(
            or_(
                func.lower(Equipment.name).like(pattern),
                func.lower(func.coalesce(Equipment.category, "")).like(pattern),
            )
        )
    return query.order_by(
        case((Equipment.id == NO_EQUIPMENT_ID, 0), else_=1),
        Equipment.is_active.desc(),
        func.lower(Equipment.name).asc(),
    ).all()


def get_equipment(equipment_id: int) -> Equipment:
    equipment = db.session.get(Equipment, equipment_id)
    if equipment is None:
        raise ApiError.not_found("Equipment not found")
    return equipment


def equipment_name_exists(name: str, excluding_id: int | None = None) -> bool:
    query = db.session.query(Equipment).filter(
        func.lower(func.trim(Equipment.name)) == clean_name(name).lower()
    )
    if excluding_id is not None:
        query = query.filter(Equipment.id != excluding_id)
    return query.first() is not None


def create_equipment(name: object, category: object = None) -> Equipment:
    cleaned = required_name(name, "Equipment name")
    if equipment_name_exists(cleaned):
        raise ApiError.conflict("Equipment with that name already exists")
    equipment = Equipment(
        name=cleaned,
        category=optional_text(category, "Category"),
        is_active=True,
    )
    db.session.add(equipment)
    db.session.commit()
    return equipment


def update_equipment(
    equipment_id: int,
    name: object,
    category: object,
    is_active: bool,
) -> Equipment:
    equipment = get_equipment(equipment_id)
    cleaned = required_name(name, "Equipment name")
    if not isinstance(is_active, bool):
        raise ApiError.validation("Active must be true or false")
    if equipment_id == NO_EQUIPMENT_ID and (
        not is_active or cleaned != "No Equipment"
    ):
        raise ApiError.validation(
            "The reserved No Equipment record must remain active and keep its name"
        )
    if equipment_name_exists(cleaned, excluding_id=equipment_id):
        raise ApiError.conflict("Equipment with that name already exists")
    equipment.name = cleaned
    equipment.category = optional_text(category, "Category")
    equipment.is_active = is_active
    db.session.commit()
    return equipment


def set_equipment_active(equipment_id: int, is_active: bool) -> Equipment:
    if not isinstance(is_active, bool):
        raise ApiError.validation("Active must be true or false")
    if equipment_id == NO_EQUIPMENT_ID and not is_active:
        raise ApiError.validation("No Equipment is reserved and cannot be deactivated")
    equipment = get_equipment(equipment_id)
    equipment.is_active = is_active
    db.session.commit()
    return equipment


def quick_create_exercise(name: object) -> Exercise:
    return create_exercise(name, "", "Other", [])


def quick_create_equipment(name: object) -> Equipment:
    return create_equipment(name, "")


def quick_create_muscle_group(name: object) -> MuscleGroup:
    return create_muscle_group(name, "")

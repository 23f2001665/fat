from __future__ import annotations

from sqlalchemy import func

from ..extensions import db
from .dates import now_timestamp
from .models import (
    AppMetadata,
    Equipment,
    Exercise,
    ExerciseMuscle,
    MuscleGroup,
    User,
)
from .text import normalize_name


MUSCLE_GROUPS: list[tuple[str, str]] = [
    ("Chest", "Pectoralis major"),
    ("Upper Chest", "Clavicular pectoralis"),
    ("Triceps", "Elbow extensors"),
    ("Biceps", "Elbow flexors"),
    ("Brachialis", "Elbow flexor beneath the biceps"),
    ("Forearms", "Wrist and grip musculature"),
    ("Front Delts", "Anterior deltoid"),
    ("Side Delts", "Lateral deltoid"),
    ("Rear Delts", "Posterior deltoid"),
    ("Upper Back", "Traps, rhomboids and lats"),
    ("Lower Back", "Erector spinae"),
    ("Quadriceps", "Knee extensors"),
    ("Hamstrings", "Knee flexors and hip extensors"),
    ("Glutes", "Hip extensors"),
    ("Calves", "Ankle plantar flexors"),
    ("Abs", "Trunk flexors and anti-extension"),
]

EQUIPMENT: list[tuple[int, str, str]] = [
    (2, "Barbell", "Free weights"),
    (3, "Dumbbell", "Free weights"),
    (4, "Cable Machine", "Machines"),
    (5, "Smith Machine", "Machines"),
    (6, "Leg Press Machine", "Machines"),
    (7, "Leg Extension Machine", "Machines"),
    (8, "Resistance Band", "Portable"),
    (9, "Other", "Other"),
]

EXERCISES: list[tuple[str, str, list[tuple[str, str]]]] = [
    (
        "Barbell Back Squat",
        "Compound",
        [
            ("Quadriceps", "Primary"),
            ("Glutes", "Secondary"),
            ("Lower Back", "Secondary"),
            ("Abs", "Secondary"),
        ],
    ),
    (
        "Leg Press",
        "Compound",
        [("Quadriceps", "Primary"), ("Glutes", "Secondary"), ("Hamstrings", "Secondary")],
    ),
    ("Leg Extension", "Isolation", [("Quadriceps", "Primary")]),
    (
        "Romanian Deadlift",
        "Compound",
        [("Hamstrings", "Primary"), ("Glutes", "Primary"), ("Lower Back", "Secondary")],
    ),
    (
        "Bench Press",
        "Compound",
        [("Chest", "Primary"), ("Triceps", "Secondary"), ("Front Delts", "Secondary")],
    ),
    (
        "Incline Dumbbell Press",
        "Compound",
        [("Upper Chest", "Primary"), ("Triceps", "Secondary"), ("Front Delts", "Secondary")],
    ),
    (
        "Overhead Press",
        "Compound",
        [("Front Delts", "Primary"), ("Triceps", "Secondary"), ("Side Delts", "Secondary")],
    ),
    ("Lateral Raise", "Isolation", [("Side Delts", "Primary")]),
    (
        "Pull Up",
        "Bodyweight",
        [("Upper Back", "Primary"), ("Biceps", "Secondary"), ("Forearms", "Secondary")],
    ),
    (
        "Barbell Row",
        "Compound",
        [
            ("Upper Back", "Primary"),
            ("Biceps", "Secondary"),
            ("Rear Delts", "Secondary"),
            ("Lower Back", "Secondary"),
        ],
    ),
    ("Lat Pulldown", "Compound", [("Upper Back", "Primary"), ("Biceps", "Secondary")]),
    ("Face Pull", "Isolation", [("Rear Delts", "Primary"), ("Upper Back", "Secondary")]),
    ("Barbell Curl", "Isolation", [("Biceps", "Primary"), ("Forearms", "Secondary")]),
    (
        "Hammer Curl",
        "Isolation",
        [("Brachialis", "Primary"), ("Biceps", "Secondary"), ("Forearms", "Secondary")],
    ),
    ("Triceps Pushdown", "Isolation", [("Triceps", "Primary")]),
    ("Skull Crusher", "Isolation", [("Triceps", "Primary")]),
    (
        "Dips",
        "Bodyweight",
        [("Chest", "Primary"), ("Triceps", "Secondary"), ("Front Delts", "Secondary")],
    ),
    ("Standing Calf Raise", "Isolation", [("Calves", "Primary")]),
    (
        "Hanging Leg Raise",
        "Bodyweight",
        [("Abs", "Primary"), ("Forearms", "Secondary")],
    ),
    ("Plank", "Bodyweight", [("Abs", "Primary"), ("Lower Back", "Secondary")]),
    ("Hip Thrust", "Compound", [("Glutes", "Primary"), ("Hamstrings", "Secondary")]),
    ("Walking Lunge", "Compound", [("Quadriceps", "Primary"), ("Glutes", "Secondary")]),
    ("Cable Fly", "Isolation", [("Chest", "Primary"), ("Front Delts", "Secondary")]),
    ("Rear Delt Fly", "Isolation", [("Rear Delts", "Primary"), ("Upper Back", "Secondary")]),
]

M_KEY = "seed.muscle_groups.v1"
E_KEY = "seed.equipment.v1"
X_KEY = "seed.exercises.v1"
U_KEY = "seed.local_user.v1"


def _marker_present(key: str) -> bool:
    return db.session.get(AppMetadata, key) is not None


def _set_marker(key: str) -> None:
    marker = db.session.get(AppMetadata, key)
    if marker is None:
        db.session.add(AppMetadata(key=key, value=now_timestamp()))
    else:
        marker.value = now_timestamp()


def _find_muscle(name: str) -> MuscleGroup | None:
    return (
        db.session.query(MuscleGroup)
        .filter(func.lower(func.trim(MuscleGroup.name)) == name.strip().lower())
        .first()
    )


def _find_equipment_by_name(name: str) -> Equipment | None:
    return (
        db.session.query(Equipment)
        .filter(func.lower(func.trim(Equipment.name)) == name.strip().lower())
        .first()
    )


def ensure_reserved_equipment() -> None:
    """No Equipment (id 1) must always exist, stay active and keep its name."""

    reserved = db.session.get(Equipment, 1)
    if reserved is None:
        db.session.add(
            Equipment(id=1, name="No Equipment", category="Reserved", is_active=True)
        )
        db.session.flush()
        return
    if reserved.name != "No Equipment":
        raise RuntimeError(
            "Equipment id 1 must be the reserved 'No Equipment' record, "
            f"found '{reserved.name}'"
        )
    if not reserved.is_active:
        raise RuntimeError("The reserved 'No Equipment' record must stay active")


def seed_database() -> None:
    """Insert the predefined catalogue and local user exactly once per marker."""

    ensure_reserved_equipment()

    if not _marker_present(M_KEY):
        for name, description in MUSCLE_GROUPS:
            if _find_muscle(name) is None:
                db.session.add(
                    MuscleGroup(name=name, description=description, is_active=True)
                )
        db.session.flush()
        _set_marker(M_KEY)

    if not _marker_present(E_KEY):
        for equipment_id, name, category in EQUIPMENT:
            if db.session.get(Equipment, equipment_id) is None and _find_equipment_by_name(name) is None:
                db.session.add(
                    Equipment(id=equipment_id, name=name, category=category, is_active=True)
                )
        db.session.flush()
        _set_marker(E_KEY)

    if not _marker_present(X_KEY):
        for name, category, muscles in EXERCISES:
            normalized = normalize_name(name)
            existing = (
                db.session.query(Exercise)
                .filter(Exercise.normalized_name == normalized)
                .first()
            )
            if existing is None:
                exercise = Exercise(
                    name=name,
                    normalized_name=normalized,
                    description=None,
                    category=category,
                    is_active=True,
                )
                db.session.add(exercise)
                db.session.flush()
                for muscle_name, role in muscles:
                    muscle = _find_muscle(muscle_name)
                    if muscle is None:
                        continue
                    db.session.add(
                        ExerciseMuscle(
                            exercise_id=exercise.id,
                            muscle_group_id=muscle.id,
                            role=role,
                        )
                    )
        db.session.flush()
        _set_marker(X_KEY)

    if not _marker_present(U_KEY):
        if db.session.get(User, 1) is None:
            now = now_timestamp()
            db.session.add(
                User(
                    id=1,
                    name="Athlete",
                    height_cm=None,
                    date_of_birth=None,
                    created_at=now,
                    updated_at=now,
                )
            )
        _set_marker(U_KEY)

    db.session.commit()

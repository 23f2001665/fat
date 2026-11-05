from __future__ import annotations

import pytest
from sqlalchemy import text
from sqlalchemy.exc import IntegrityError

from backend.application.data_seed import seed_database
from backend.application.errors import ApiError
from backend.application.services import library_service


def test_seed_counts_and_idempotence(ctx):
    assert len(library_service.list_muscle_groups(True)) == 16
    assert len(library_service.list_equipment(True)) == 9
    assert len(library_service.list_exercises(True)) == 24

    seed_database()

    assert len(library_service.list_muscle_groups(True)) == 16
    assert len(library_service.list_equipment(True)) == 9
    assert len(library_service.list_exercises(True)) == 24


def test_no_equipment_is_reserved_and_database_protected(ctx):
    reserved = library_service.get_equipment(1)
    assert reserved.name == "No Equipment"
    assert reserved.is_active is True

    with pytest.raises(ApiError):
        library_service.set_equipment_active(1, False)
    with pytest.raises(ApiError):
        library_service.update_equipment(1, "Something else", "Reserved", True)

    db = __import__("backend.extensions", fromlist=["db"]).db
    with pytest.raises(IntegrityError):
        db.session.execute(text("DELETE FROM equipment WHERE id = 1"))
        db.session.commit()
    db.session.rollback()
    assert library_service.get_equipment(1).name == "No Equipment"


def test_exercise_names_are_unique_case_and_space_insensitively(ctx):
    created = library_service.create_exercise("  Cable   Press ", "", "Other", [])
    assert created.name == "Cable Press"

    with pytest.raises(ApiError) as error:
        library_service.create_exercise("cable press", "", "Other", [])
    assert error.value.status_code == 409


def test_muscle_group_used_by_exercise_cannot_be_deactivated(ctx):
    chest = next(
        row for row in library_service.list_muscle_groups(True) if row.name == "Chest"
    )
    with pytest.raises(ApiError):
        library_service.set_muscle_group_active(chest.id, False)

    olive = library_service.create_muscle_group("Obliques", "Rotational trunk")
    deactivated = library_service.set_muscle_group_active(olive.id, False)
    assert deactivated.is_active is False


def test_assignments_reject_duplicates_and_inactive_muscles(ctx):
    chest = next(
        row for row in library_service.list_muscle_groups(True) if row.name == "Chest"
    )
    with pytest.raises(ApiError):
        library_service.create_exercise(
            "Test Press",
            "",
            "Compound",
            [
                {"muscle_group_id": chest.id, "role": "Primary"},
                {"muscle_group_id": chest.id, "role": "Secondary"},
            ],
        )


def test_categories_are_validated(ctx):
    with pytest.raises(ApiError):
        library_service.create_exercise("Odd Lift", "", "Explosive", [])
    lift = library_service.create_exercise("Odd Lift", "", "", [])
    assert lift.category == "Other"


def test_reserved_equipment_cannot_be_deleted_through_api(client):
    response = client.patch("/api/library/equipment/1/active", json={"is_active": False})
    assert response.status_code == 400
    response = client.put(
        "/api/library/equipment/1",
        json={"name": "No Equipment", "category": "Reserved", "is_active": False},
    )
    assert response.status_code == 400


def test_search_and_include_inactive(ctx):
    library_service.set_exercise_active(1, False)
    active = library_service.list_exercises(False)
    assert all(row.id != 1 for row in active)
    all_exercises = library_service.list_exercises(True)
    assert any(row.id == 1 and row.is_active is False for row in all_exercises)
    found = library_service.list_exercises(True, "barbell")
    assert all("barbell" in row.name.lower() or "barbell" in (row.description or "").lower() for row in found)

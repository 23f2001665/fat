from __future__ import annotations

from datetime import timedelta

from sqlalchemy import text

from backend.application.dates import today_local, week_start
from backend.application.errors import ApiError
from backend.application.services import workout_service


def _exercise_ids():
    from backend.application.services import library_service

    rows = library_service.list_exercises(True)
    return {row.name: row.id for row in rows}


def test_seeded_set_sequence_continues_and_deletion_does_not_renumber(ctx, client):
    exercises = _exercise_ids()
    today = today_local()
    first = client.post(
        "/api/workouts",
        json={
            "date": today.isoformat(),
            "exercise_id": exercises["Barbell Back Squat"],
            "equipment_id": 2,
            "weight": 100,
            "complete_reps": 5,
            "partial_reps": 1,
        },
    )
    assert first.status_code == 201
    assert first.get_json()["set"]["sequence_number"] == 1
    assert first.get_json()["set"]["volume"] == 550.0

    second = client.post(
        "/api/workouts",
        json={
            "date": today.isoformat(),
            "exercise_id": exercises["Leg Extension"],
            "equipment_id": 7,
            "weight": 40,
            "complete_reps": 10,
            "partial_reps": 0,
        },
    )
    assert second.status_code == 201
    assert second.get_json()["set"]["sequence_number"] == 2

    delete = client.delete(f"/api/workouts/{today.isoformat()}/1")
    assert delete.status_code == 200
    day = client.get("/api/workouts", query_string={"date": today.isoformat()}).get_json()
    assert [row["sequence_number"] for row in day["sets"]] == [2]

    third = client.post(
        "/api/workouts",
        json={
            "date": today.isoformat(),
            "exercise_id": exercises["Leg Extension"],
            "equipment_id": 7,
            "weight": 45,
            "complete_reps": 8,
            "partial_reps": 0,
        },
    )
    assert third.get_json()["set"]["sequence_number"] == 3


def test_future_workouts_and_invalid_values_are_rejected(ctx, client):
    exercises = _exercise_ids()
    payload = {
        "date": (today_local() + timedelta(days=1)).isoformat(),
        "exercise_id": exercises["Bench Press"],
        "equipment_id": 2,
        "weight": 60,
        "complete_reps": 8,
        "partial_reps": 0,
    }
    assert client.post("/api/workouts", json=payload).status_code == 400

    payload["date"] = today_local().isoformat()
    payload["weight"] = -1
    assert client.post("/api/workouts", json=payload).status_code == 400
    payload["weight"] = 60
    payload["rir"] = 11
    assert client.post("/api/workouts", json=payload).status_code == 400
    payload.pop("rir")
    payload["complete_reps"] = 0
    payload["partial_reps"] = 0
    assert client.post("/api/workouts", json=payload).status_code == 400


def test_history_is_one_row_per_day_with_names_in_first_performed_order(ctx, client):
    exercises = _exercise_ids()
    today = today_local()
    yesterday = today - timedelta(days=1)
    for workout_date, exercise, equipment in (
        (today, "Bench Press", 2),
        (today, "Cable Fly", 4),
        (yesterday, "Barbell Back Squat", 2),
    ):
        response = client.post(
            "/api/workouts",
            json={
                "date": workout_date.isoformat(),
                "exercise_id": exercises[exercise],
                "equipment_id": equipment,
                "weight": 50,
                "complete_reps": 10,
                "partial_reps": 0,
            },
        )
        assert response.status_code == 201
    days = client.get("/api/workouts/history").get_json()["days"]
    assert len(days) == 2
    assert days[0]["date"] == today.isoformat()
    assert days[0]["exercise_names"] == ["Bench Press", "Cable Fly"]
    assert days[0]["set_count"] == 2
    assert days[0]["total_volume"] == 1000.0


def test_inactive_exercise_can_remain_in_existing_set_but_not_a_new_one(ctx, client):
    exercises = _exercise_ids()
    today = today_local()
    exercise_id = exercises["Bench Press"]
    created = client.post(
        "/api/workouts",
        json={
            "date": today.isoformat(),
            "exercise_id": exercise_id,
            "equipment_id": 2,
            "weight": 60,
            "complete_reps": 8,
            "partial_reps": 0,
        },
    )
    assert created.status_code == 201
    toggled = client.patch(
        f"/api/library/exercises/{exercise_id}/active", json={"is_active": False}
    )
    assert toggled.status_code == 200

    rejected = client.post(
        "/api/workouts",
        json={
            "date": today.isoformat(),
            "exercise_id": exercise_id,
            "equipment_id": 2,
            "weight": 60,
            "complete_reps": 8,
            "partial_reps": 0,
        },
    )
    assert rejected.status_code == 400

    edited = client.put(
        f"/api/workouts/{today.isoformat()}/1",
        json={
            "exercise_id": exercise_id,
            "equipment_id": 2,
            "weight": 62.5,
            "complete_reps": 8,
            "partial_reps": 1,
            "rir": 2,
            "extra_info": "felt strong",
        },
    )
    assert edited.status_code == 200
    assert edited.get_json()["set"]["weight"] == 62.5


def test_workout_service_rejects_unreasonable_weight(ctx):
    exercises = _exercise_ids()
    try:
        workout_service.add_set(
            1, today_local(), exercises["Bench Press"], 2, 2001, 5, 0
        )
    except ApiError as error:
        assert error.status_code == 400
    else:
        raise AssertionError("Expected validation error")


def test_week_label_starts_on_monday():
    today = today_local()
    assert week_start(today).weekday() == 0

from __future__ import annotations

from datetime import timedelta

from backend.application.dates import today_local, week_start
from backend.application.services import analytics_service, library_service, workout_service


def _catalogue():
    exercises = {row.name: row.id for row in library_service.list_exercises(True)}
    muscles = {row.name: row.id for row in library_service.list_muscle_groups(True)}
    return exercises, muscles


def _add(workout_date, exercise_id, equipment_id, weight, complete, partial=0, rir=2):
    return workout_service.add_set(
        workout_date,
        exercise_id,
        equipment_id,
        weight,
        complete,
        partial,
        rest_seconds=120,
        rir=rir,
    )


def _seed_workouts():
    exercises, muscles = _catalogue()
    today = today_local()
    start = today - timedelta(days=20)
    _add(start, exercises["Barbell Back Squat"], 2, 100, 5)
    _add(start, exercises["Barbell Back Squat"], 2, 110, 3)
    _add(start + timedelta(days=1), exercises["Bench Press"], 2, 80, 8)
    _add(start + timedelta(days=7), exercises["Barbell Back Squat"], 2, 120, 2)
    return exercises, muscles, start, today


def test_exercise_metrics_use_actual_sets(ctx):
    exercises, _, start, today = _seed_workouts()
    report = analytics_service.build_report(
        "all",
        exercise_id=exercises["Barbell Back Squat"],
        anchor=today,
    )
    assert report["exercise"]["total_sets"] == 3
    assert report["exercise"]["total_volume"] == 100 * 5 + 110 * 3 + 120 * 2
    assert report["exercise"]["total_effective_reps"] == 10
    assert report["exercise"]["max_weight"] == 120
    assert len(report["exercise"]["weight_progression"]) == 2
    assert report["exercise"]["weight_progression"][0]["date"] == start.isoformat()
    assert report["exercise"]["weight_progression"][0]["value"] == 110
    assert report["has_data"] is True


def test_primary_volume_is_full_and_secondary_is_half(ctx):
    exercises, _, _, today = _seed_workouts()
    report = analytics_service.build_report(
        "all",
        exercise_id=exercises["Barbell Back Squat"],
        anchor=today,
    )
    total = report["exercise"]["total_volume"]
    quads = next(row for row in report["muscle_groups"] if row["name"] == "Quadriceps")
    glutes = next(row for row in report["muscle_groups"] if row["name"] == "Glutes")
    assert quads["primary_volume"] == total
    assert quads["secondary_volume"] == 0
    assert glutes["primary_volume"] == 0
    assert glutes["secondary_volume"] == total * 0.5
    assert quads["primary_sets"] == 3
    assert glutes["secondary_sets"] == 3
    assert quads["workout_days"] == 2


def test_muscle_filter_applies_to_all_metrics(ctx):
    exercises, muscles, _, today = _seed_workouts()
    report = analytics_service.build_report(
        "all",
        muscle_group_id=muscles["Chest"],
        anchor=today,
    )
    assert report["exercise"]["total_sets"] == 1
    assert report["exercise"]["exercise_id"] is None
    assert [row["label"] for row in report["exercise_frequency"]] == ["Bench Press"]
    assert all(row["name"] == "Chest" for row in report["muscle_groups"])


def test_weekly_volume_uses_monday_and_omits_empty_weeks(ctx):
    _, _, start, today = _seed_workouts()
    report = analytics_service.build_report("all", anchor=today)
    assert len(report["weekly_volume"]) == 2
    assert report["weekly_volume"][0]["date"] == week_start(start).isoformat()
    assert report["weekly_volume"][1]["date"] == week_start(
        start + timedelta(days=7)
    ).isoformat()
    expected = 100 * 5 + 110 * 3 + 120 * 2 + 80 * 8
    assert sum(point["value"] for point in report["weekly_volume"]) == expected


def test_period_and_custom_range_filter_records(ctx):
    _, _, start, today = _seed_workouts()
    recent = analytics_service.build_report("7", anchor=today)
    # Only records within the inclusive seven-day window count.
    assert all(
        point["date"] >= (today - timedelta(days=6)).isoformat()
        for point in recent["exercise"]["weight_progression"]
    )

    custom = analytics_service.build_report(
        "custom",
        start=start,
        end=start,
        anchor=today,
    )
    assert custom["exercise"]["total_sets"] == 2
    assert custom["range"]["start"] == start.isoformat()
    assert custom["range"]["end"] == start.isoformat()


def test_empty_period_has_no_synthetic_points(ctx):
    today = today_local()
    report = analytics_service.build_report(
        "custom",
        start=today - timedelta(days=60),
        end=today - timedelta(days=50),
        anchor=today,
    )
    assert report["has_data"] is False
    assert report["exercise"]["total_sets"] == 0
    assert report["exercise"]["weight_progression"] == []
    assert report["weekly_volume"] == []
    assert report["muscle_groups"] == []


def test_rir_trend_ignores_missing_rir(ctx):
    exercises, _, _, today = _seed_workouts()
    _add(today - timedelta(days=2), exercises["Barbell Back Squat"], 2, 90, 5, rir=None)
    report = analytics_service.build_report(
        "all",
        exercise_id=exercises["Barbell Back Squat"],
        anchor=today,
    )
    assert all(point["value"] == 2 for point in report["exercise"]["rir_trend"])


def test_analytics_api_shape(client):
    response = client.get("/api/analytics", query_string={"period": "all"})
    assert response.status_code == 200
    payload = response.get_json()
    assert payload["period"] == "all"
    assert "muscle_groups" in payload
    assert "weekly_volume" in payload

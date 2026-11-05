from __future__ import annotations

from datetime import timedelta

import pytest

from backend.application.calculations import bmi
from backend.application.dates import today_local
from backend.application.errors import ApiError
from backend.application.services import profile_service


def test_bmi_uses_values_from_the_same_measurement_snapshot(ctx):
    today = today_local()
    profile_service.add_measurement(today - timedelta(days=30), weight_kg=80)
    profile_service.add_measurement(today - timedelta(days=20), height_cm=180)
    metrics = profile_service.latest_metrics()
    assert metrics["weight_kg"] == 80
    assert metrics["height_cm"] == 180
    assert metrics["bmi"] is None

    latest_date = today - timedelta(days=10)
    profile_service.add_measurement(latest_date, weight_kg=75, height_cm=180)
    metrics = profile_service.latest_metrics()
    assert metrics["bmi"] == bmi(75, 180)
    assert metrics["bmi_measured_at"] == latest_date.isoformat()


def test_measurements_are_append_only_and_partial_snapshots_are_allowed(ctx):
    today = today_local()
    profile_service.add_measurement(today - timedelta(days=2), weight_kg=80)
    profile_service.add_measurement(today - timedelta(days=1), waist_cm=82)
    profile_service.update_profile("Alex", 180, None)

    history = profile_service.measurement_history()
    assert len(history) == 2
    assert history[0].waist_cm == 82
    assert history[1].weight_kg == 80
    metrics = profile_service.latest_metrics()
    assert metrics["weight_kg"] == 80
    assert metrics["waist_cm"] == 82
    assert profile_service.get_local_user().height_cm == 180


def test_empty_and_invalid_measurements_are_rejected(ctx):
    today = today_local()
    with pytest.raises(ApiError):
        profile_service.add_measurement(today)
    with pytest.raises(ApiError):
        profile_service.add_measurement(today, body_fat_percentage=101)
    with pytest.raises(ApiError):
        profile_service.add_measurement(today + timedelta(days=1), weight_kg=70)
    with pytest.raises(ApiError):
        profile_service.add_measurement(today, waist_cm=501)
    with pytest.raises(ApiError):
        profile_service.add_measurement(today, weight_kg=0)


def test_comparisons_use_latest_and_previous_observation_per_metric(ctx):
    today = today_local()
    profile_service.add_measurement(today - timedelta(days=20), weight_kg=80, arm_cm=35)
    profile_service.add_measurement(today - timedelta(days=10), weight_kg=79)
    profile_service.add_measurement(today - timedelta(days=5), arm_cm=35.5)
    comparisons = {row["metric"]: row for row in profile_service.comparisons()}
    assert comparisons["weight"]["current"] == 79
    assert comparisons["weight"]["previous"] == 80
    assert comparisons["weight"]["delta"] == -1
    assert comparisons["arm"]["current"] == 35.5
    assert comparisons["arm"]["previous"] == 35


def test_metric_series_contains_actual_dates_only(ctx):
    today = today_local()
    profile_service.add_measurement(today - timedelta(days=8), weight_kg=80)
    profile_service.add_measurement(today - timedelta(days=4), weight_kg=79)
    profile_service.add_measurement(today - timedelta(days=1), height_cm=180)

    from backend.application.dates import DateRange

    points = profile_service.metric_series("weight", DateRange())
    assert [row["value"] for row in points] == [80, 79]
    assert [row["date"] for row in points] == [
        (today - timedelta(days=8)).isoformat(),
        (today - timedelta(days=4)).isoformat(),
    ]
    assert profile_service.metric_series("bmi", DateRange()) == []


def test_profile_api_does_not_create_measurement(client):
    today = today_local()
    response = client.put(
        "/api/profile",
        json={"name": "Sam", "height_cm": 175, "date_of_birth": "1990-05-04"},
    )
    assert response.status_code == 200
    profile = client.get("/api/profile").get_json()
    assert profile["user"]["name"] == "Sam"
    assert profile["measurement_count"] == 0
    assert profile["user"]["height_cm"] == 175


def test_measurement_api_returns_derived_bmi(client):
    today = today_local()
    response = client.post(
        "/api/profile/measurements",
        json={
            "measured_at": today.isoformat(),
            "weight_kg": 72,
            "height_cm": 180,
            "notes": "morning",
        },
    )
    assert response.status_code == 201
    payload = response.get_json()["measurement"]
    assert round(payload["bmi"], 4) == round(bmi(72, 180), 4)
    assert payload["bmi_category"] == "Healthy range"

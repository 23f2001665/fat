from __future__ import annotations

from backend.application.dates import today_local


def _exercise_ids(client):
    payload = client.get("/api/library/exercises").get_json()
    return {row["name"]: row["id"] for row in payload["exercises"]}


def test_anonymous_requests_are_rejected_and_health_is_public(anonymous_client):
    response = anonymous_client.get("/api/workouts")
    assert response.status_code == 401
    assert response.get_json()["code"] == "authentication_required"

    assert anonymous_client.get("/api/health").status_code == 200
    assert anonymous_client.get("/api/auth/me").status_code == 401


def test_login_logout_and_current_user(anonymous_client):
    response = anonymous_client.post(
        "/api/auth/login",
        json={"username": " ADMIN ", "password": "admin123"},
    )
    assert response.status_code == 200
    assert response.get_json()["user"]["username"] == "admin"
    assert response.get_json()["user"]["role"] == "admin"

    me = anonymous_client.get("/api/auth/me")
    assert me.status_code == 200
    assert me.get_json()["user"]["username"] == "admin"

    assert anonymous_client.post("/api/auth/logout").status_code == 200
    assert anonymous_client.get("/api/auth/me").status_code == 401


def test_login_rejects_bad_credentials(anonymous_client):
    assert (
        anonymous_client.post(
            "/api/auth/login",
            json={"username": "admin", "password": "wrong-password"},
        ).status_code
        == 401
    )
    assert (
        anonymous_client.post(
            "/api/auth/login",
            json={"username": "missing", "password": "admin123"},
        ).status_code
        == 401
    )
    assert (
        anonymous_client.post(
            "/api/auth/login",
            json={"username": "admin"},
        ).status_code
        == 401
    )


def test_normal_user_can_read_but_not_mutate_the_library(user_client):
    assert user_client.get("/api/library/exercises").status_code == 200

    forbidden = (
        ("post", "/api/library/exercises", {"name": "Cable Press", "muscles": []}),
        ("put", "/api/library/exercises/1", {"name": "Renamed"}),
        ("patch", "/api/library/exercises/1/active", {"is_active": False}),
        ("post", "/api/library/muscle-groups", {"name": "New Muscle"}),
        ("patch", "/api/library/muscle-groups/1/active", {"is_active": False}),
        ("post", "/api/library/equipment", {"name": "New Machine"}),
        ("put", "/api/library/equipment/1", {"name": "No Equipment"}),
        ("patch", "/api/library/equipment/1/active", {"is_active": False}),
    )
    for method, path, payload in forbidden:
        response = getattr(user_client, method)(path, json=payload)
        assert response.status_code == 403, (method, path, response.get_json())


def test_workouts_measurements_and_analytics_are_isolated(admin_client, user_client):
    exercises = _exercise_ids(admin_client)
    today = today_local().isoformat()
    admin_set = admin_client.post(
        "/api/workouts",
        json={
            "date": today,
            "exercise_id": exercises["Bench Press"],
            "equipment_id": 2,
            "weight": 100,
            "complete_reps": 5,
            "partial_reps": 0,
        },
    )
    assert admin_set.status_code == 201
    user_set = user_client.post(
        "/api/workouts",
        json={
            "date": today,
            "exercise_id": exercises["Leg Extension"],
            "equipment_id": 7,
            "weight": 40,
            "complete_reps": 10,
            "partial_reps": 0,
        },
    )
    assert user_set.status_code == 201

    # Sequence numbers restart for every account.
    assert admin_set.get_json()["set"]["sequence_number"] == 1
    assert user_set.get_json()["set"]["sequence_number"] == 1

    admin_day = admin_client.get("/api/workouts", query_string={"date": today}).get_json()
    user_day = user_client.get("/api/workouts", query_string={"date": today}).get_json()
    assert [row["exercise_name"] for row in admin_day["sets"]] == ["Bench Press"]
    assert [row["exercise_name"] for row in user_day["sets"]] == ["Leg Extension"]

    # The same day/sequence pair belongs to different users.
    assert user_client.delete(f"/api/workouts/{today}/1").status_code == 200
    assert admin_client.get("/api/workouts", query_string={"date": today}).get_json()[
        "set_count"
    ] == 1

    assert user_client.get("/api/workouts/history").get_json()["days"] == []

    admin_client.post(
        "/api/profile/measurements",
        json={"measured_at": today, "weight_kg": 100},
    )
    user_client.post(
        "/api/profile/measurements",
        json={"measured_at": today, "weight_kg": 70},
    )
    admin_profile = admin_client.get("/api/profile").get_json()
    user_profile = user_client.get("/api/profile").get_json()
    assert admin_profile["latest_metrics"]["weight_kg"] == 100
    assert user_profile["latest_metrics"]["weight_kg"] == 70
    assert admin_profile["measurement_count"] == 1
    assert user_profile["measurement_count"] == 1

    admin_report = admin_client.get(
        "/api/analytics", query_string={"period": "all"}
    ).get_json()
    user_report = user_client.get(
        "/api/analytics", query_string={"period": "all"}
    ).get_json()
    assert admin_report["exercise"]["total_sets"] == 1
    assert user_report["exercise"]["total_sets"] == 0
    assert admin_report["exercise"]["total_volume"] == 500.0


def test_user_management_and_last_admin_protection(
    admin_client, user_client, anonymous_client
):
    assert user_client.get("/api/admin/users").status_code == 403

    created = admin_client.post(
        "/api/admin/users",
        json={
            "name": "Coach",
            "username": "coach",
            "password": "coach1234",
            "role": "user",
        },
    )
    assert created.status_code == 201
    coach_id = created.get_json()["user"]["id"]

    assert (
        admin_client.post(
            "/api/admin/users",
            json={
                "name": "Duplicate",
                "username": " COACH ",
                "password": "coach1234",
            },
        ).status_code
        == 409
    )

    coach = anonymous_client
    assert (
        coach.post(
            "/api/auth/login",
            json={"username": "coach", "password": "coach1234"},
        ).status_code
        == 200
    )
    assert coach.get("/api/admin/users").status_code == 403

    updated = admin_client.put(
        f"/api/admin/users/{coach_id}",
        json={
            "name": "Coach Two",
            "role": "user",
            "is_active": False,
        },
    )
    assert updated.status_code == 200
    assert updated.get_json()["user"]["is_active"] is False

    # Deactivation revokes the existing session as well as future logins.
    assert coach.get("/api/workouts").status_code == 401
    assert (
        anonymous_client.post(
            "/api/auth/login",
            json={"username": "coach", "password": "coach1234"},
        ).status_code
        == 401
    )

    # The signed-in administrator cannot demote or deactivate themselves.
    admin_id = admin_client.get("/api/auth/me").get_json()["user"]["id"]
    assert (
        admin_client.put(
            f"/api/admin/users/{admin_id}",
            json={"name": "Administrator", "role": "user", "is_active": True},
        ).status_code
        == 400
    )
    assert (
        admin_client.put(
            f"/api/admin/users/{admin_id}",
            json={"name": "Administrator", "role": "admin", "is_active": False},
        ).status_code
        == 400
    )

    # A second administrator can deactivate the original one.
    root = admin_client.post(
        "/api/admin/users",
        json={
            "name": "Root",
            "username": "root",
            "password": "root1234",
            "role": "admin",
        },
    )
    assert root.status_code == 201
    root_client = anonymous_client
    assert (
        root_client.post(
            "/api/auth/login",
            json={"username": "root", "password": "root1234"},
        ).status_code
        == 200
    )
    assert (
        root_client.put(
            f"/api/admin/users/{admin_id}",
            json={"name": "Administrator", "role": "admin", "is_active": False},
        ).status_code
        == 200
    )
    assert admin_client.get("/api/workouts").status_code == 401


def test_password_change(user_client, anonymous_client):
    assert (
        user_client.put(
            "/api/auth/password",
            json={
                "current_password": "wrong-password",
                "new_password": "athlete456",
            },
        ).status_code
        == 400
    )
    assert (
        user_client.put(
            "/api/auth/password",
            json={
                "current_password": "athlete123",
                "new_password": "athlete456",
            },
        ).status_code
        == 200
    )
    assert anonymous_client.post("/api/auth/logout").status_code == 200
    assert (
        anonymous_client.post(
            "/api/auth/login",
            json={"username": "athlete", "password": "athlete123"},
        ).status_code
        == 401
    )
    assert (
        anonymous_client.post(
            "/api/auth/login",
            json={"username": "athlete", "password": "athlete456"},
        ).status_code
        == 200
    )

from __future__ import annotations

from collections import OrderedDict
from datetime import date

from sqlalchemy import text

from ..dates import DateRange, format_date, resolve_period, today_local, week_start
from ..errors import ApiError
from ...extensions import db
from ..models import Exercise, MuscleGroup


SET_FILTER = """
    (:start_date IS NULL OR w.date >= :start_date)
    AND (:end_date IS NULL OR w.date <= :end_date)
    AND (:exercise_id IS NULL OR w.exercise_id = :exercise_id)
    AND (:muscle_group_id IS NULL OR EXISTS (
        SELECT 1 FROM exercise_muscles emf
        WHERE emf.exercise_id = w.exercise_id
          AND emf.muscle_group_id = :muscle_group_id
    ))
"""


def _params(
    date_range: DateRange,
    exercise_id: int | None,
    muscle_group_id: int | None,
) -> dict:
    return {
        "start_date": format_date(date_range.start) if date_range.start else None,
        "end_date": format_date(date_range.end) if date_range.end else None,
        "exercise_id": exercise_id,
        "muscle_group_id": muscle_group_id,
    }


def _summary(date_range: DateRange, exercise_id, muscle_group_id) -> dict:
    row = db.session.execute(
        text(
            f"""
            WITH sets AS (
                SELECT w.weight,
                       w.complete_reps + w.partial_reps / 2.0 AS effective_reps,
                       w.weight * (w.complete_reps + w.partial_reps / 2.0) AS volume,
                       w.rir
                FROM workout_logs w
                WHERE {SET_FILTER}
            )
            SELECT COUNT(*),
                   COALESCE(SUM(effective_reps), 0),
                   COALESCE(SUM(volume), 0),
                   MAX(weight),
                   AVG(weight),
                   AVG(rir)
            FROM sets
            """
        ),
        _params(date_range, exercise_id, muscle_group_id),
    ).one()
    return {
        "total_sets": int(row[0]),
        "total_effective_reps": float(row[1] or 0.0),
        "total_volume": float(row[2] or 0.0),
        "max_weight": float(row[3]) if row[3] is not None else None,
        "average_weight": float(row[4]) if row[4] is not None else None,
        "average_rir": float(row[5]) if row[5] is not None else None,
    }


def _per_date(date_range: DateRange, exercise_id, muscle_group_id) -> list[dict]:
    rows = db.session.execute(
        text(
            f"""
            SELECT w.date,
                   MAX(w.weight),
                   SUM(w.weight * (w.complete_reps + w.partial_reps / 2.0)),
                   AVG(w.rir)
            FROM workout_logs w
            WHERE {SET_FILTER}
            GROUP BY w.date
            ORDER BY w.date ASC
            """
        ),
        _params(date_range, exercise_id, muscle_group_id),
    ).all()
    return [
        {
            "date": row[0] if isinstance(row[0], date) else date.fromisoformat(str(row[0])),
            "max_weight": float(row[1]),
            "volume": float(row[2]),
            "average_rir": float(row[3]) if row[3] is not None else None,
        }
        for row in rows
    ]


def _muscle_groups(date_range: DateRange, exercise_id, muscle_group_id) -> list[dict]:
    rows = db.session.execute(
        text(
            f"""
            WITH sets AS (
                SELECT w.date, w.exercise_id,
                       w.weight * (w.complete_reps + w.partial_reps / 2.0) AS volume
                FROM workout_logs w
                WHERE {SET_FILTER}
            )
            SELECT mg.id, mg.name,
                   SUM(CASE WHEN em.role = 'Primary' THEN 1 ELSE 0 END) AS primary_sets,
                   SUM(CASE WHEN em.role = 'Secondary' THEN 1 ELSE 0 END) AS secondary_sets,
                   COALESCE(SUM(CASE WHEN em.role = 'Primary' THEN s.volume ELSE 0 END), 0),
                   COALESCE(SUM(CASE WHEN em.role = 'Secondary' THEN s.volume ELSE 0 END), 0) * 0.5,
                   COUNT(DISTINCT s.date)
            FROM sets s
            JOIN exercise_muscles em ON em.exercise_id = s.exercise_id
            JOIN muscle_groups mg ON mg.id = em.muscle_group_id
            WHERE (:muscle_group_id IS NULL OR mg.id = :muscle_group_id)
            GROUP BY mg.id, mg.name
            ORDER BY (primary_sets + secondary_sets) DESC, mg.name COLLATE NOCASE
            """
        ),
        _params(date_range, exercise_id, muscle_group_id),
    ).all()
    return [
        {
            "muscle_group_id": int(row[0]),
            "name": row[1],
            "primary_sets": int(row[2]),
            "secondary_sets": int(row[3]),
            "primary_volume": float(row[4] or 0.0),
            "secondary_volume": float(row[5] or 0.0),
            "total_volume": float(row[4] or 0.0) + float(row[5] or 0.0),
            "workout_days": int(row[6]),
        }
        for row in rows
    ]


def _exercise_frequency(date_range: DateRange, exercise_id, muscle_group_id) -> list[dict]:
    rows = db.session.execute(
        text(
            f"""
            SELECT e.id, e.name, COUNT(*)
            FROM workout_logs w
            JOIN exercises e ON e.id = w.exercise_id
            WHERE {SET_FILTER}
            GROUP BY e.id, e.name
            ORDER BY COUNT(*) DESC, e.name COLLATE NOCASE
            """
        ),
        _params(date_range, exercise_id, muscle_group_id),
    ).all()
    return [{"label": row[1], "value": float(row[2])} for row in rows]


def build_report(
    period: str | None,
    start: date | None = None,
    end: date | None = None,
    exercise_id: int | None = None,
    muscle_group_id: int | None = None,
    anchor: date | None = None,
) -> dict:
    key, date_range, label = resolve_period(period, start, end, anchor or today_local())

    exercise_name = None
    if exercise_id is not None:
        exercise = db.session.get(Exercise, exercise_id)
        if exercise is None:
            raise ApiError.not_found("Exercise not found")
        exercise_name = exercise.name

    muscle_group_name = None
    if muscle_group_id is not None:
        muscle = db.session.get(MuscleGroup, muscle_group_id)
        if muscle is None:
            raise ApiError.not_found("Muscle group not found")
        muscle_group_name = muscle.name

    summary = _summary(date_range, exercise_id, muscle_group_id)
    per_date = _per_date(date_range, exercise_id, muscle_group_id)
    muscle_groups = _muscle_groups(date_range, exercise_id, muscle_group_id)
    exercise_frequency = _exercise_frequency(date_range, exercise_id, muscle_group_id)

    weight_progression = [
        {"date": row["date"].isoformat(), "value": row["max_weight"]}
        for row in per_date
    ]
    volume_progression = [
        {"date": row["date"].isoformat(), "value": row["volume"]}
        for row in per_date
    ]
    rir_trend = [
        {"date": row["date"].isoformat(), "value": row["average_rir"]}
        for row in per_date
        if row["average_rir"] is not None
    ]

    weekly: OrderedDict[date, float] = OrderedDict()
    for row in per_date:
        monday = week_start(row["date"])
        weekly[monday] = weekly.get(monday, 0.0) + row["volume"]
    weekly_volume = [
        {"date": monday.isoformat(), "value": value}
        for monday, value in sorted(weekly.items())
    ]

    return {
        "period": key,
        "period_label": label,
        "range": {
            "start": format_date(date_range.start) if date_range.start else None,
            "end": format_date(date_range.end) if date_range.end else None,
        },
        "filter": {
            "exercise_id": exercise_id,
            "exercise_name": exercise_name,
            "muscle_group_id": muscle_group_id,
            "muscle_group_name": muscle_group_name,
        },
        "exercise": {
            "exercise_id": exercise_id,
            "exercise_name": exercise_name,
            **summary,
            "weight_progression": weight_progression,
            "volume_progression": volume_progression,
            "rir_trend": rir_trend,
        },
        "muscle_groups": muscle_groups,
        "weekly_volume": weekly_volume,
        "muscle_distribution": [
            {"label": row["name"], "value": row["total_volume"]}
            for row in muscle_groups
        ],
        "exercise_frequency": exercise_frequency,
        "has_data": summary["total_sets"] > 0,
    }

from __future__ import annotations


def effective_reps(complete_reps: int, partial_reps: int) -> float:
    """Partial reps are half reps, so two partial reps equal one full rep."""

    return float(complete_reps) + float(partial_reps) / 2.0


def set_volume(weight_kg: float, complete_reps: int, partial_reps: int) -> float:
    """Training volume in kilograms for one set."""

    return float(weight_kg) * effective_reps(complete_reps, partial_reps)


def bmi(weight_kg: float | None, height_cm: float | None) -> float | None:
    """BMI from a single weight/height pair.

    Callers must pass values from the same measurement snapshot. Missing or
    non-positive values never produce a synthetic BMI.
    """

    if weight_kg is None or height_cm is None:
        return None
    if weight_kg <= 0 or height_cm <= 0:
        return None
    height_m = float(height_cm) / 100.0
    return float(weight_kg) / (height_m * height_m)


def bmi_category(value: float | None) -> str:
    if value is None:
        return "Unavailable"
    if value < 18.5:
        return "Underweight"
    if value < 25.0:
        return "Healthy range"
    if value < 30.0:
        return "Overweight"
    return "Obese range"

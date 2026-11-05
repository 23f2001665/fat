from __future__ import annotations

from backend.application.calculations import bmi, bmi_category, effective_reps, set_volume


def test_partial_reps_are_half_reps():
    assert effective_reps(8, 0) == 8.0
    assert effective_reps(8, 1) == 8.5
    assert effective_reps(8, 2) == 9.0
    assert effective_reps(8, 3) == 9.5


def test_volume_uses_effective_reps():
    assert set_volume(30, 6, 0) == 180.0
    assert set_volume(30, 6, 1) == 195.0
    assert set_volume(0, 10, 2) == 0.0


def test_bmi_requires_both_same_snapshot_values():
    assert bmi(80, 180) == 80 / (1.8 * 1.8)
    assert bmi(None, 180) is None
    assert bmi(80, None) is None
    assert bmi(0, 180) is None
    assert bmi(80, 0) is None


def test_bmi_categories():
    assert bmi_category(None) == "Unavailable"
    assert bmi_category(18.4) == "Underweight"
    assert bmi_category(18.5) == "Healthy range"
    assert bmi_category(25) == "Overweight"
    assert bmi_category(30) == "Obese range"

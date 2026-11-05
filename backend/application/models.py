from __future__ import annotations

from datetime import date, datetime

from sqlalchemy import CheckConstraint, Index, UniqueConstraint, func
from sqlalchemy.orm import relationship

from ..extensions import db


class User(db.Model):
    __tablename__ = "users"

    id = db.Column(db.Integer, primary_key=True)
    name = db.Column(db.String(120), nullable=False)
    height_cm = db.Column(db.Float, nullable=True)
    date_of_birth = db.Column(db.Date, nullable=True)
    created_at = db.Column(db.String(64), nullable=False)
    updated_at = db.Column(db.String(64), nullable=False)

    measurements = relationship("BodyMeasurement", back_populates="user")

    __table_args__ = (
        CheckConstraint("height_cm IS NULL OR height_cm > 0", name="user_height_positive"),
    )


class MuscleGroup(db.Model):
    __tablename__ = "muscle_groups"

    id = db.Column(db.Integer, primary_key=True)
    name = db.Column(db.String(120), nullable=False, unique=True)
    description = db.Column(db.Text, nullable=True)
    is_active = db.Column(db.Boolean, nullable=False, default=True, server_default="1")

    exercise_links = relationship("ExerciseMuscle", back_populates="muscle_group")

    __table_args__ = (
        CheckConstraint("is_active IN (0, 1)", name="muscle_group_active_boolean"),
        Index(
            "muscle_groups_name_nocase",
            func.lower(func.trim(name)),
            unique=True,
        ),
    )


class Equipment(db.Model):
    __tablename__ = "equipment"

    id = db.Column(db.Integer, primary_key=True)
    name = db.Column(db.String(120), nullable=False, unique=True)
    category = db.Column(db.String(120), nullable=True)
    is_active = db.Column(db.Boolean, nullable=False, default=True, server_default="1")

    workout_logs = relationship("WorkoutLog", back_populates="equipment")

    __table_args__ = (
        CheckConstraint("is_active IN (0, 1)", name="equipment_active_boolean"),
        Index(
            "equipment_name_nocase",
            func.lower(func.trim(name)),
            unique=True,
        ),
    )


class Exercise(db.Model):
    __tablename__ = "exercises"

    id = db.Column(db.Integer, primary_key=True)
    name = db.Column(db.String(120), nullable=False)
    normalized_name = db.Column(db.String(120), nullable=False, unique=True)
    description = db.Column(db.Text, nullable=True)
    category = db.Column(db.String(32), nullable=False, default="Other", server_default="Other")
    is_active = db.Column(db.Boolean, nullable=False, default=True, server_default="1")

    muscle_links = relationship(
        "ExerciseMuscle",
        back_populates="exercise",
        cascade="all, delete-orphan",
        order_by="ExerciseMuscle.role, ExerciseMuscle.muscle_group_id",
    )
    workout_logs = relationship("WorkoutLog", back_populates="exercise")

    __table_args__ = (
        CheckConstraint(
            "category IN ('Compound', 'Isolation', 'Bodyweight', 'Other')",
            name="exercise_category_valid",
        ),
        CheckConstraint("is_active IN (0, 1)", name="exercise_active_boolean"),
    )


class ExerciseMuscle(db.Model):
    __tablename__ = "exercise_muscles"

    exercise_id = db.Column(
        db.Integer,
        db.ForeignKey("exercises.id", ondelete="RESTRICT"),
        primary_key=True,
    )
    muscle_group_id = db.Column(
        db.Integer,
        db.ForeignKey("muscle_groups.id", ondelete="RESTRICT"),
        primary_key=True,
    )
    role = db.Column(db.String(16), nullable=False)

    exercise = relationship("Exercise", back_populates="muscle_links")
    muscle_group = relationship("MuscleGroup", back_populates="exercise_links")

    __table_args__ = (
        CheckConstraint("role IN ('Primary', 'Secondary')", name="exercise_muscle_role_valid"),
    )


class WorkoutLog(db.Model):
    __tablename__ = "workout_logs"

    date = db.Column(db.Date, primary_key=True)
    sequence_number = db.Column(db.Integer, primary_key=True)
    exercise_id = db.Column(
        db.Integer,
        db.ForeignKey("exercises.id", ondelete="RESTRICT"),
        nullable=False,
    )
    equipment_id = db.Column(
        db.Integer,
        db.ForeignKey("equipment.id", ondelete="RESTRICT"),
        nullable=False,
    )
    weight = db.Column(db.Float, nullable=False, default=0.0)
    complete_reps = db.Column(db.Integer, nullable=False, default=0)
    partial_reps = db.Column(db.Integer, nullable=False, default=0)
    rest_seconds = db.Column(db.Integer, nullable=True)
    rir = db.Column(db.Integer, nullable=True)
    extra_info = db.Column(db.Text, nullable=True)

    exercise = relationship("Exercise", back_populates="workout_logs")
    equipment = relationship("Equipment", back_populates="workout_logs")

    __table_args__ = (
        CheckConstraint("weight >= 0", name="workout_weight_non_negative"),
        CheckConstraint("complete_reps >= 0", name="workout_complete_reps_non_negative"),
        CheckConstraint("partial_reps >= 0", name="workout_partial_reps_non_negative"),
        CheckConstraint(
            "complete_reps + partial_reps > 0",
            name="workout_has_reps",
        ),
        CheckConstraint(
            "rest_seconds IS NULL OR rest_seconds >= 0",
            name="workout_rest_non_negative",
        ),
        CheckConstraint(
            "rir IS NULL OR (rir >= 0 AND rir <= 10)",
            name="workout_rir_range",
        ),
        Index("workout_logs_exercise_idx", "exercise_id"),
        Index("workout_logs_equipment_idx", "equipment_id"),
        Index("workout_logs_date_idx", "date"),
    )

    @property
    def effective_reps(self) -> float:
        return self.complete_reps + self.partial_reps / 2.0

    @property
    def volume(self) -> float:
        return self.weight * self.effective_reps


class BodyMeasurement(db.Model):
    __tablename__ = "body_measurements"

    id = db.Column(db.Integer, primary_key=True)
    user_id = db.Column(
        db.Integer,
        db.ForeignKey("users.id", ondelete="RESTRICT"),
        nullable=False,
    )
    measured_at = db.Column(db.Date, nullable=False)
    weight_kg = db.Column(db.Float, nullable=True)
    height_cm = db.Column(db.Float, nullable=True)
    body_fat_percentage = db.Column(db.Float, nullable=True)
    waist_cm = db.Column(db.Float, nullable=True)
    chest_cm = db.Column(db.Float, nullable=True)
    hip_cm = db.Column(db.Float, nullable=True)
    arm_cm = db.Column(db.Float, nullable=True)
    thigh_cm = db.Column(db.Float, nullable=True)
    notes = db.Column(db.Text, nullable=True)

    user = relationship("User", back_populates="measurements")

    __table_args__ = (
        CheckConstraint("weight_kg IS NULL OR weight_kg > 0", name="measurement_weight_positive"),
        CheckConstraint("height_cm IS NULL OR height_cm > 0", name="measurement_height_positive"),
        CheckConstraint(
            "body_fat_percentage IS NULL OR "
            "(body_fat_percentage >= 0 AND body_fat_percentage <= 100)",
            name="measurement_body_fat_range",
        ),
        CheckConstraint(
            "waist_cm IS NULL OR waist_cm >= 0",
            name="measurement_waist_non_negative",
        ),
        CheckConstraint(
            "chest_cm IS NULL OR chest_cm >= 0",
            name="measurement_chest_non_negative",
        ),
        CheckConstraint(
            "hip_cm IS NULL OR hip_cm >= 0",
            name="measurement_hip_non_negative",
        ),
        CheckConstraint(
            "arm_cm IS NULL OR arm_cm >= 0",
            name="measurement_arm_non_negative",
        ),
        CheckConstraint(
            "thigh_cm IS NULL OR thigh_cm >= 0",
            name="measurement_thigh_non_negative",
        ),
        Index("body_measurements_user_date_idx", "user_id", "measured_at"),
    )

    @property
    def bmi(self) -> float | None:
        from .calculations import bmi

        return bmi(self.weight_kg, self.height_cm)


class AppMetadata(db.Model):
    __tablename__ = "app_metadata"

    key = db.Column(db.String(120), primary_key=True)
    value = db.Column(db.Text, nullable=False)


class SchemaMigration(db.Model):
    __tablename__ = "schema_migrations"

    version = db.Column(db.Integer, primary_key=True)
    description = db.Column(db.Text, nullable=False)
    applied_at = db.Column(db.String(64), nullable=False)


def utc_now() -> str:
    """Local timestamps are friendlier for a single-user desktop app."""

    return datetime.now().astimezone().isoformat(timespec="seconds")


def date_key(value: date) -> str:
    return value.isoformat()

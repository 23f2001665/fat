from __future__ import annotations

import math

from .errors import ApiError


def normalize_name(value: str) -> str:
    return " ".join(value.split()).lower()


def clean_name(value: str) -> str:
    return " ".join(value.split())


def required_name(value: object, field: str) -> str:
    if not isinstance(value, str):
        raise ApiError.validation(f"{field} is required")
    cleaned = clean_name(value)
    if not cleaned:
        raise ApiError.validation(f"{field} cannot be empty")
    if len(cleaned) > 120:
        raise ApiError.validation(f"{field} cannot be longer than 120 characters")
    return cleaned


def optional_text(value: object, field: str) -> str | None:
    if value is None:
        return None
    if not isinstance(value, str):
        raise ApiError.validation(f"{field} must be text")
    trimmed = value.strip()
    if not trimmed:
        return None
    if len(trimmed) > 4000:
        raise ApiError.validation(f"{field} cannot be longer than 4000 characters")
    return trimmed


def finite_number(value: object, field: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ApiError.validation(f"{field} must be a number")
    number = float(value)
    if not math.isfinite(number):
        raise ApiError.validation(f"{field} must be a finite number")
    return number


def optional_number(value: object, field: str) -> float | None:
    if value is None or (isinstance(value, str) and not value.strip()):
        return None
    return finite_number(value, field)


def whole_number(value: object, field: str) -> int:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ApiError.validation(f"{field} must be a whole number")
    if isinstance(value, float) and not value.is_integer():
        raise ApiError.validation(f"{field} must be a whole number")
    return int(value)


def optional_whole_number(value: object, field: str) -> int | None:
    if value is None or (isinstance(value, str) and not value.strip()):
        return None
    return whole_number(value, field)


def positive_identifier(value: object, field: str) -> int:
    identifier = whole_number(value, field)
    if identifier <= 0:
        raise ApiError.validation(f"{field} is invalid")
    return identifier

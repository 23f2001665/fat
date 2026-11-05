from __future__ import annotations

from flask import request

from ..dates import parse_date, parse_optional_date
from ..errors import ApiError
from ..text import finite_number, optional_number, optional_whole_number, positive_identifier


def json_object() -> dict:
    data = request.get_json(silent=True)
    if not isinstance(data, dict):
        raise ApiError.validation("Request body must be a JSON object")
    return data


def payload_date(data: dict, field: str = "date"):
    if field not in data or data[field] in (None, ""):
        raise ApiError.validation(f"{field.replace('_', ' ').title()} is required")
    return parse_date(data[field], field.replace("_", " ").title())


def payload_optional_date(data: dict, field: str):
    return parse_optional_date(data.get(field), field.replace("_", " ").title())


def payload_int(data: dict, field: str, label: str | None = None) -> int:
    if field not in data:
        raise ApiError.validation(f"{label or field.replace('_', ' ').title()} is required")
    return positive_identifier(data[field], label or field.replace("_", " ").title())


def payload_optional_int(data: dict, field: str, label: str | None = None) -> int | None:
    value = data.get(field)
    if value is None or value == "":
        return None
    return optional_whole_number(value, label or field.replace("_", " ").title())


def payload_float(data: dict, field: str, label: str | None = None) -> float:
    if field not in data or data[field] is None or data[field] == "":
        raise ApiError.validation(f"{label or field.replace('_', ' ').title()} is required")
    return finite_number(data[field], label or field.replace("_", " ").title())


def payload_optional_float(data: dict, field: str, label: str | None = None) -> float | None:
    return optional_number(data.get(field), label or field.replace("_", " ").title())


def query_bool(name: str, default: bool = False) -> bool:
    raw = request.args.get(name)
    if raw is None:
        return default
    normalized = raw.strip().lower()
    if normalized in {"1", "true", "yes", "on"}:
        return True
    if normalized in {"0", "false", "no", "off"}:
        return False
    raise ApiError.validation(f"{name} must be true or false")


def query_optional_int(name: str, label: str | None = None) -> int | None:
    raw = request.args.get(name)
    if raw is None or not raw.strip():
        return None
    try:
        value = int(raw)
    except ValueError as exc:
        raise ApiError.validation(f"{label or name.replace('_', ' ').title()} must be a whole number") from exc
    if value <= 0:
        raise ApiError.validation(f"{label or name.replace('_', ' ').title()} is invalid")
    return value


def query_optional_date(name: str, label: str):
    raw = request.args.get(name)
    if raw is None or not raw.strip():
        return None
    return parse_date(raw, label)

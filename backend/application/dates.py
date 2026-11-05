from __future__ import annotations

import calendar

from dataclasses import dataclass
from datetime import date, datetime, timedelta

from .errors import ApiError


DATE_FORMAT = "%Y-%m-%d"


def today_local() -> date:
    return date.today()


def now_timestamp() -> str:
    return datetime.now().astimezone().isoformat(timespec="seconds")


def format_date(value: date) -> str:
    return value.strftime(DATE_FORMAT)


def parse_date(value: object, field: str = "Date") -> date:
    if isinstance(value, date) and not isinstance(value, datetime):
        return value
    if not isinstance(value, str) or not value.strip():
        raise ApiError.validation(f"{field} is required (YYYY-MM-DD)")
    try:
        return datetime.strptime(value.strip(), DATE_FORMAT).date()
    except ValueError as exc:
        raise ApiError.validation(f"{field} must be a valid date (YYYY-MM-DD)") from exc


def parse_optional_date(value: object, field: str) -> date | None:
    if value is None or (isinstance(value, str) and not value.strip()):
        return None
    return parse_date(value, field)


def display_date(value: date) -> str:
    return value.strftime("%d %b %Y")


def add_months(value: date, months: int) -> date:
    month_index = value.year * 12 + (value.month - 1) + months
    year, month0 = divmod(month_index, 12)
    month = month0 + 1
    last_day = calendar.monthrange(year, month)[1]
    return date(year, month, min(value.day, last_day))


def week_start(value: date) -> date:
    return value - timedelta(days=value.weekday())


@dataclass(frozen=True)
class DateRange:
    start: date | None = None
    end: date | None = None

    def contains(self, value: date) -> bool:
        return (self.start is None or value >= self.start) and (
            self.end is None or value <= self.end
        )


PERIOD_LABELS = {
    "7": "Last 7 days",
    "30": "Last 30 days",
    "90": "Last 90 days",
    "6m": "Last 6 months",
    "12m": "Last 12 months",
    "all": "All time",
}


def resolve_period(
    period: str | None,
    start: date | None = None,
    end: date | None = None,
    today: date | None = None,
) -> tuple[str, DateRange, str]:
    key = (period or "30").strip()
    anchor = today or today_local()

    if key == "custom":
        if start is None and end is None:
            raise ApiError.validation("Choose both custom dates and apply the range")
        if start and end and start > end:
            raise ApiError.validation("The start date must be on or before the end date")
        return key, DateRange(start, end), _custom_label(start, end)

    if key not in PERIOD_LABELS:
        raise ApiError.validation(f"Unknown reporting period '{key}'")

    if key in {"7", "30", "90"}:
        days = int(key)
        result = DateRange(anchor - timedelta(days=days - 1), anchor)
    elif key in {"6m", "12m"}:
        months = 6 if key == "6m" else 12
        result = DateRange(add_months(anchor, -months), anchor)
    else:
        result = DateRange(None, None)

    return key, result, PERIOD_LABELS[key]


def _custom_label(start: date | None, end: date | None) -> str:
    if start and end:
        return f"{format_date(start)} to {format_date(end)}"
    if start:
        return f"From {format_date(start)}"
    if end:
        return f"Until {format_date(end)}"
    return "Custom range"

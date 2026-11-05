from __future__ import annotations

from dataclasses import dataclass


@dataclass
class ApiError(Exception):
    """An error that is safe and useful to expose through the JSON API."""

    message: str
    status_code: int = 400
    code: str = "validation_error"

    def __post_init__(self) -> None:
        super().__init__(self.message)

    @classmethod
    def validation(cls, message: str) -> "ApiError":
        return cls(message, 400, "validation_error")

    @classmethod
    def not_found(cls, message: str) -> "ApiError":
        return cls(message, 404, "not_found")

    @classmethod
    def conflict(cls, message: str) -> "ApiError":
        return cls(message, 409, "conflict")

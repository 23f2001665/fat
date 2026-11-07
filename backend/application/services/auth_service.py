from __future__ import annotations

import re

from sqlalchemy import func

from werkzeug.security import check_password_hash, generate_password_hash

from ..dates import now_timestamp
from ..errors import ApiError
from ...extensions import db
from ..models import User

USER_ROLES = ("admin", "user")
USERNAME_PATTERN = re.compile(r"^[a-z0-9][a-z0-9._-]{2,39}$")
MIN_PASSWORD_LENGTH = 8
MAX_PASSWORD_LENGTH = 128


def normalize_username(value: object) -> str:
    if not isinstance(value, str):
        raise ApiError.validation("Username is required")
    username = value.strip().lower()
    if not USERNAME_PATTERN.fullmatch(username):
        raise ApiError.validation(
            "Username must be 3–40 characters using letters, numbers, dots, "
            "hyphens or underscores"
        )
    return username


def validate_password(value: object, field: str = "Password") -> str:
    if not isinstance(value, str):
        raise ApiError.validation(f"{field} is required")
    if len(value) < MIN_PASSWORD_LENGTH:
        raise ApiError.validation(
            f"{field} must be at least {MIN_PASSWORD_LENGTH} characters"
        )
    if len(value) > MAX_PASSWORD_LENGTH:
        raise ApiError.validation(
            f"{field} cannot be longer than {MAX_PASSWORD_LENGTH} characters"
        )
    return value


def validate_role(value: object) -> str:
    if value not in USER_ROLES:
        raise ApiError.validation("Role must be admin or user")
    return str(value)


def _display_name(value: object) -> str:
    from ..text import required_name

    return required_name(value, "Name")


def find_by_username(username: object) -> User | None:
    if not isinstance(username, str) or not username.strip():
        return None
    normalized = username.strip().lower()
    return (
        db.session.query(User)
        .filter(func.lower(func.trim(User.username)) == normalized)
        .first()
    )


def get_user(user_id: int) -> User:
    user = db.session.get(User, user_id)
    if user is None:
        raise ApiError.not_found("User not found")
    return user


def authenticate(username: object, password: object) -> User:
    user = find_by_username(username)
    valid = (
        user is not None
        and isinstance(password, str)
        and check_password_hash(user.password_hash, password)
    )
    if not valid or not user.is_active:
        raise ApiError.unauthorized("Invalid username or password")
    return user


def password_hash(password: object, field: str = "Password") -> str:
    return generate_password_hash(validate_password(password, field))


def change_password(user: User, current_password: object, new_password: object) -> None:
    if not isinstance(current_password, str) or not check_password_hash(
        user.password_hash, current_password
    ):
        raise ApiError.validation("Current password is incorrect")
    if current_password == new_password:
        raise ApiError.validation("The new password must be different")
    user.password_hash = password_hash(new_password, "New password")
    user.updated_at = now_timestamp()
    db.session.commit()


def list_users() -> list[User]:
    return (
        db.session.query(User)
        .order_by(
            func.lower(func.trim(User.username)).asc(),
        )
        .all()
    )


def _active_admin_count(excluding_user_id: int | None = None) -> int:
    query = db.session.query(func.count(User.id)).filter(
        User.role == "admin",
        User.is_active.is_(True),
    )
    if excluding_user_id is not None:
        query = query.filter(User.id != excluding_user_id)
    return int(query.scalar() or 0)


def create_user(
    name: object,
    username: object,
    password: object,
    role: object = "user",
) -> User:
    cleaned_username = normalize_username(username)
    if find_by_username(cleaned_username) is not None:
        raise ApiError.conflict("That username is already in use")
    cleaned_role = validate_role(role)
    now = now_timestamp()
    user = User(
        name=_display_name(name),
        username=cleaned_username,
        password_hash=password_hash(password),
        role=cleaned_role,
        is_active=True,
        created_at=now,
        updated_at=now,
    )
    db.session.add(user)
    db.session.commit()
    return user


def update_user(
    actor: User,
    user_id: int,
    name: object,
    role: object,
    is_active: object,
    new_password: object = None,
) -> User:
    user = get_user(user_id)
    cleaned_role = validate_role(role)
    if not isinstance(is_active, bool):
        raise ApiError.validation("Active must be true or false")
    if user.id == actor.id and (cleaned_role != "admin" or not is_active):
        raise ApiError.validation(
            "You cannot demote or deactivate your own account"
        )
    if (
        user.role == "admin"
        and user.is_active
        and (cleaned_role != "admin" or not is_active)
        and _active_admin_count(excluding_user_id=user.id) == 0
    ):
        raise ApiError.validation("At least one active administrator is required")

    user.name = _display_name(name)
    user.role = cleaned_role
    user.is_active = is_active
    if new_password not in (None, ""):
        user.password_hash = password_hash(new_password, "New password")
    user.updated_at = now_timestamp()
    db.session.commit()
    return user


def user_count() -> int:
    return int(db.session.query(func.count(User.id)).scalar() or 0)

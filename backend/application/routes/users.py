from __future__ import annotations

from flask import Blueprint, jsonify

from ..auth import admin_required, current_user
from ..serializers import serialize_user
from ..services import auth_service
from .common import json_object


bp = Blueprint("users", __name__)


@bp.get("/admin/users")
@admin_required
def list_users():
    return jsonify(
        {"users": [serialize_user(user) for user in auth_service.list_users()]}
    )


@bp.post("/admin/users")
@admin_required
def create_user():
    data = json_object()
    user = auth_service.create_user(
        data.get("name"),
        data.get("username"),
        data.get("password"),
        data.get("role", "user"),
    )
    return jsonify({"message": "User created", "user": serialize_user(user)}), 201


@bp.put("/admin/users/<int:user_id>")
@admin_required
def update_user(user_id: int):
    data = json_object()
    user = auth_service.update_user(
        current_user(),
        user_id,
        data.get("name"),
        data.get("role"),
        data.get("is_active"),
        data.get("password"),
    )
    return jsonify({"message": "User updated", "user": serialize_user(user)})

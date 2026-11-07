from __future__ import annotations

from flask import Blueprint, jsonify, request, session

from ..auth import current_user
from ..serializers import serialize_user
from ..services import auth_service
from .common import json_object


bp = Blueprint("auth", __name__)


@bp.post("/auth/login")
def login():
    data = json_object()
    user = auth_service.authenticate(data.get("username"), data.get("password"))
    session.clear()
    session.permanent = True
    session["user_id"] = user.id
    return jsonify({"message": "Signed in", "user": serialize_user(user)})


@bp.post("/auth/logout")
def logout():
    session.clear()
    return jsonify({"message": "Signed out"})


@bp.get("/auth/me")
def me():
    return jsonify({"user": serialize_user(current_user())})


@bp.put("/auth/password")
def change_password():
    data = json_object()
    auth_service.change_password(
        current_user(),
        data.get("current_password"),
        data.get("new_password"),
    )
    return jsonify({"message": "Password changed"})

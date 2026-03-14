from __future__ import annotations

from typing import Any, Dict

from pydantic import BaseModel, Field


class AgentEnvelope(BaseModel):
    type: str
    request_id: str
    session_id: str
    user_id: str
    payload: Dict[str, Any] = Field(default_factory=dict)


class ToolCallPayload(BaseModel):
    tool_call_id: str
    tool_name: str
    arguments: Dict[str, Any] = Field(default_factory=dict)


class ToolResultPayload(BaseModel):
    tool_call_id: str
    tool_name: str
    ok: bool
    result: Dict[str, Any] = Field(default_factory=dict)
    error: str | None = None

from __future__ import annotations

import asyncio
from dataclasses import dataclass
from typing import Any, Awaitable, Callable


ToolInvoker = Callable[[str, dict[str, Any]], Awaitable[dict[str, Any]]]
EventEmitter = Callable[[str, str, str, str, dict[str, Any]], Awaitable[None]]


@dataclass(frozen=True)
class ToolContext:
    request_id: str
    session_id: str
    user_id: str


class ToolRuntime:
    def __init__(
        self,
        *,
        context: ToolContext,
        invoker: ToolInvoker,
        emit: EventEmitter,
        timeout_seconds: float,
    ):
        self._context = context
        self._invoker = invoker
        self._emit = emit
        self._timeout_seconds = timeout_seconds

    async def call(self, tool_name: str, arguments: dict[str, Any]) -> dict[str, Any]:
        await self._emit(
            "tool.started",
            self._context.request_id,
            self._context.session_id,
            self._context.user_id,
            {"toolName": tool_name, "arguments": arguments},
        )
        result = await asyncio.wait_for(
            self._invoker(tool_name, arguments), timeout=self._timeout_seconds
        )
        await self._emit(
            "tool.completed",
            self._context.request_id,
            self._context.session_id,
            self._context.user_id,
            {"toolName": tool_name, "result": result},
        )
        return result

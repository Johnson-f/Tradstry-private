from __future__ import annotations

import asyncio
from dataclasses import dataclass
from typing import Awaitable, Callable, Literal, overload

from tradstry_agents.schemas import (
    EventEmitter,
    EmptyToolArguments,
    GraphEventType,
    JsonPayload,
    LimitToolArguments,
    ToolArguments,
    ToolCompletedPayload,
    ToolName,
    ToolStartedPayload,
)

ToolInvoker = Callable[[ToolName, ToolArguments], Awaitable[JsonPayload]]


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

    @overload
    async def call(
        self,
        tool_name: Literal[
            "account_summary",
            "positions",
            "analytics_snapshot",
            "playbook_setups",
        ],
        arguments: EmptyToolArguments,
    ) -> JsonPayload:
        ...

    @overload
    async def call(
        self,
        tool_name: Literal["recent_trades", "journal_entries", "notebook_context"],
        arguments: LimitToolArguments,
    ) -> JsonPayload:
        ...

    async def call(self, tool_name: ToolName, arguments: ToolArguments) -> JsonPayload:
        await self._emit(
            GraphEventType.TOOL_STARTED.value,
            self._context.request_id,
            self._context.session_id,
            self._context.user_id,
            ToolStartedPayload(tool_name=tool_name, arguments=arguments).model_dump(by_alias=True),
        )
        result = await asyncio.wait_for(
            self._invoker(tool_name, arguments), timeout=self._timeout_seconds
        )
        await self._emit(
            GraphEventType.TOOL_COMPLETED.value,
            self._context.request_id,
            self._context.session_id,
            self._context.user_id,
            ToolCompletedPayload(tool_name=tool_name, result=result).model_dump(by_alias=True),
        )
        return result

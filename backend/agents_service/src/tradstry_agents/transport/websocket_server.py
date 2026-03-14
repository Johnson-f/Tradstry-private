from __future__ import annotations

import asyncio
import contextlib
import json
from contextlib import asynccontextmanager
from typing import Any
from uuid import uuid4

import uvicorn
from fastapi import FastAPI, WebSocket, WebSocketDisconnect

from tradstry_agents.config import load_settings
from tradstry_agents.graphs import AgentGraphRunner
from tradstry_agents.memory import OpenVikingMemoryStore
from tradstry_agents.providers import GroqChatProvider, OpenRouterEmbeddingProvider
from tradstry_agents.prompts import PromptLibrary
from tradstry_agents.schemas import AgentEnvelope
from tradstry_agents.tools import ToolContext, ToolRuntime


class AgentServiceRuntime:
    def __init__(self):
        self.settings = load_settings()
        self.embedding_provider = OpenRouterEmbeddingProvider(self.settings)
        self.chat_provider = GroqChatProvider(self.settings)
        self.memory_store = OpenVikingMemoryStore(self.settings, self.embedding_provider)
        self.prompt_library = PromptLibrary(self.settings)

    async def startup(self) -> None:
        await self.memory_store.initialize()

    async def shutdown(self) -> None:
        await self.memory_store.close()


class AgentConnection:
    def __init__(self, websocket: WebSocket, runtime: AgentServiceRuntime):
        self.websocket = websocket
        self.runtime = runtime
        self._request_tasks: dict[str, asyncio.Task[None]] = {}
        self._pending_tool_calls: dict[str, asyncio.Future[dict[str, Any]]] = {}

    async def handle(self) -> None:
        await self.websocket.accept()
        try:
            while True:
                raw = await self.websocket.receive_text()
                envelope = AgentEnvelope.model_validate_json(raw)
                await self._dispatch(envelope)
        except WebSocketDisconnect:
            await self._cancel_all()

    async def _dispatch(self, envelope: AgentEnvelope) -> None:
        if envelope.type == "request.start":
            task = asyncio.create_task(self._run_request(envelope))
            self._request_tasks[envelope.request_id] = task
        elif envelope.type == "request.cancel":
            task = self._request_tasks.get(envelope.request_id)
            if task is not None:
                task.cancel()
        elif envelope.type == "request.ping":
            await self._send(
                "response.pong",
                envelope.request_id,
                envelope.session_id,
                envelope.user_id,
                {},
            )
        elif envelope.type == "tool.result":
            tool_call_id = envelope.payload["toolCallId"]
            future = self._pending_tool_calls.pop(tool_call_id, None)
            if future is not None and not future.done():
                future.set_result(envelope.payload)

    async def _run_request(self, envelope: AgentEnvelope) -> None:
        try:
            message = str(envelope.payload.get("message", "")).strip()
            if not message:
                raise ValueError("request.start requires payload.message")

            await self.runtime.memory_store.append_user_turn(
                user_id=envelope.user_id,
                session_id=envelope.session_id,
                content=message,
            )

            tool_runtime = ToolRuntime(
                context=ToolContext(
                    request_id=envelope.request_id,
                    session_id=envelope.session_id,
                    user_id=envelope.user_id,
                ),
                invoker=lambda tool_name, arguments: self._call_tool(
                    envelope=envelope, tool_name=tool_name, arguments=arguments
                ),
                emit=self._send,
                timeout_seconds=self.runtime.settings.tool_timeout_seconds,
            )
            graph_runner = AgentGraphRunner(
                chat_provider=self.runtime.chat_provider,
                memory_store=self.runtime.memory_store,
                prompt_library=self.runtime.prompt_library,
                tool_runtime=tool_runtime,
                emit=self._send,
            )
            final_answer = await graph_runner.run(
                request_id=envelope.request_id,
                session_id=envelope.session_id,
                user_id=envelope.user_id,
                message=message,
            )

            for chunk in _chunk_text(final_answer):
                await self._send(
                    "response.delta",
                    envelope.request_id,
                    envelope.session_id,
                    envelope.user_id,
                    {"text": chunk},
                )

            await self.runtime.memory_store.append_assistant_turn(
                user_id=envelope.user_id,
                session_id=envelope.session_id,
                content=final_answer,
            )
            promoted = await self.runtime.memory_store.promote_memories(
                user_id=envelope.user_id,
                request_text=message,
                response_text=final_answer,
            )
            await self._send(
                "response.completed",
                envelope.request_id,
                envelope.session_id,
                envelope.user_id,
                {"text": final_answer, "promotedMemoryUris": promoted},
            )
        except asyncio.CancelledError:
            await self._send(
                "response.error",
                envelope.request_id,
                envelope.session_id,
                envelope.user_id,
                {"message": "request cancelled"},
            )
            raise
        except Exception as exc:
            await self._send(
                "response.error",
                envelope.request_id,
                envelope.session_id,
                envelope.user_id,
                {"message": str(exc)},
            )
        finally:
            self._request_tasks.pop(envelope.request_id, None)

    async def _call_tool(
        self, *, envelope: AgentEnvelope, tool_name: str, arguments: dict[str, Any]
    ) -> dict[str, Any]:
        tool_call_id = uuid4().hex
        future: asyncio.Future[dict[str, Any]] = asyncio.get_running_loop().create_future()
        self._pending_tool_calls[tool_call_id] = future
        await self._send(
            "tool.call",
            envelope.request_id,
            envelope.session_id,
            envelope.user_id,
            {
                "toolCallId": tool_call_id,
                "toolName": tool_name,
                "arguments": arguments,
            },
        )
        result = await future
        if not result.get("ok", False):
            raise RuntimeError(result.get("error") or f"{tool_name} failed")
        return dict(result.get("result", {}))

    async def _send(
        self,
        message_type: str,
        request_id: str,
        session_id: str,
        user_id: str,
        payload: dict[str, Any],
    ) -> None:
        envelope = {
            "type": message_type,
            "request_id": request_id,
            "session_id": session_id,
            "user_id": user_id,
            "payload": payload,
        }
        await self.websocket.send_text(json.dumps(envelope))

    async def _cancel_all(self) -> None:
        for task in list(self._request_tasks.values()):
            task.cancel()
        for task in list(self._request_tasks.values()):
            with contextlib.suppress(Exception):
                await task


@asynccontextmanager
async def lifespan(app: FastAPI):
    runtime = AgentServiceRuntime()
    await runtime.startup()
    app.state.runtime = runtime
    try:
        yield
    finally:
        await runtime.shutdown()


def create_app() -> FastAPI:
    settings = load_settings()
    app = FastAPI(lifespan=lifespan)

    @app.get("/healthz")
    async def healthz() -> dict[str, str]:
        return {"status": "ok"}

    @app.websocket(settings.websocket_path)
    async def agent_websocket(websocket: WebSocket) -> None:
        connection = AgentConnection(websocket, websocket.app.state.runtime)
        await connection.handle()

    return app


def run() -> None:
    settings = load_settings()
    uvicorn.run(
        "main:app",
        host=settings.host,
        port=settings.port,
        reload=False,
        log_level="info",
    )


def _chunk_text(text: str, size: int = 220) -> list[str]:
    if not text:
        return [""]
    return [text[index : index + size] for index in range(0, len(text), size)]

from __future__ import annotations

import json
from typing import Any, TypedDict

from langgraph.graph import END, StateGraph

from tradstry_agents.memory import OpenVikingMemoryStore
from tradstry_agents.providers import ChatProvider
from tradstry_agents.prompts import PromptLibrary
from tradstry_agents.tools import ToolRuntime


EventEmitter = callable


class AgentState(TypedDict, total=False):
    request_id: str
    session_id: str
    user_id: str
    message: str
    route: str
    retrieved_memory: list[dict[str, Any]]
    tool_results: dict[str, Any]
    final_answer: str


class AgentGraphRunner:
    def __init__(
        self,
        *,
        chat_provider: ChatProvider,
        memory_store: OpenVikingMemoryStore,
        prompt_library: PromptLibrary,
        tool_runtime: ToolRuntime,
        emit,
    ):
        self._chat_provider = chat_provider
        self._memory_store = memory_store
        self._prompt_library = prompt_library
        self._tool_runtime = tool_runtime
        self._emit = emit
        self._graph = self._build_graph()

    async def run(self, *, request_id: str, session_id: str, user_id: str, message: str) -> str:
        initial_state: AgentState = {
            "request_id": request_id,
            "session_id": session_id,
            "user_id": user_id,
            "message": message,
            "tool_results": {},
        }
        result = await self._graph.ainvoke(initial_state)
        return result["final_answer"]

    def _build_graph(self):
        graph = StateGraph(AgentState)
        graph.add_node("load_memory", self._load_memory)
        graph.add_node("supervisor", self._supervisor)
        graph.add_node("portfolio_analyst", self._portfolio_analyst)
        graph.add_node("journal_coach", self._journal_coach)
        graph.add_node("trading_educator", self._trading_educator)
        graph.add_node("compose", self._compose)
        graph.set_entry_point("load_memory")
        graph.add_edge("load_memory", "supervisor")
        graph.add_conditional_edges(
            "supervisor",
            lambda state: state["route"],
            {
                "portfolio_analyst": "portfolio_analyst",
                "journal_coach": "journal_coach",
                "trading_educator": "trading_educator",
            },
        )
        graph.add_edge("portfolio_analyst", "compose")
        graph.add_edge("journal_coach", "compose")
        graph.add_edge("trading_educator", "compose")
        graph.add_edge("compose", END)
        return graph.compile()

    async def _load_memory(self, state: AgentState) -> AgentState:
        retrieved = await self._memory_store.retrieve_context(
            user_id=state["user_id"],
            session_id=state["session_id"],
            query=state["message"],
        )
        payload = {
            "count": len(retrieved),
            "items": [
                {
                    "uri": item.uri,
                    "bucket": item.bucket,
                    "abstract": item.abstract,
                    "score": item.score,
                }
                for item in retrieved
            ],
        }
        await self._emit(
            "memory.retrieved",
            state["request_id"],
            state["session_id"],
            state["user_id"],
            payload,
        )
        return {
            "retrieved_memory": [
                {
                    "uri": item.uri,
                    "bucket": item.bucket,
                    "abstract": item.abstract,
                    "content": item.content,
                    "score": item.score,
                }
                for item in retrieved
            ]
        }

    async def _supervisor(self, state: AgentState) -> AgentState:
        message = state["message"].lower()
        if any(token in message for token in ("journal", "playbook", "mistake", "setup", "note")):
            route = "journal_coach"
        elif any(
            token in message
            for token in ("account", "portfolio", "position", "trade", "pnl", "win rate")
        ):
            route = "portfolio_analyst"
        else:
            route = "trading_educator"

        await self._emit(
            "route.selected",
            state["request_id"],
            state["session_id"],
            state["user_id"],
            {"route": route},
        )
        return {"route": route}

    async def _portfolio_analyst(self, state: AgentState) -> AgentState:
        message = state["message"].lower()
        results: dict[str, Any] = {
            "accountSummary": await self._tool_runtime.call("account_summary", {}),
            "positions": await self._tool_runtime.call("positions", {}),
            "recentTrades": await self._tool_runtime.call("recent_trades", {"limit": 8}),
        }
        if any(token in message for token in ("performance", "analytics", "win rate", "calendar")):
            results["analyticsSnapshot"] = await self._tool_runtime.call(
                "analytics_snapshot", {}
            )
        return {"tool_results": results}

    async def _journal_coach(self, state: AgentState) -> AgentState:
        results: dict[str, Any] = {
            "journalEntries": await self._tool_runtime.call("journal_entries", {"limit": 8}),
            "playbookSetups": await self._tool_runtime.call("playbook_setups", {}),
            "notebookContext": await self._tool_runtime.call("notebook_context", {"limit": 8}),
        }
        if "analytics" in state["message"].lower() or "win rate" in state["message"].lower():
            results["analyticsSnapshot"] = await self._tool_runtime.call(
                "analytics_snapshot", {}
            )
        return {"tool_results": results}

    async def _trading_educator(self, state: AgentState) -> AgentState:
        return {"tool_results": {"generalKnowledge": "No live account tools were required."}}

    async def _compose(self, state: AgentState) -> AgentState:
        system_prompt = self._prompt_library.compose_system_prompt(state["route"])
        user_prompt = "\n\n".join(
            [
                f"Route: {state['route']}",
                f"User message: {state['message']}",
                "Retrieved memory:",
                json.dumps(state.get("retrieved_memory", []), indent=2),
                "Tool results:",
                json.dumps(state.get("tool_results", {}), indent=2),
            ]
        )
        final_answer = await self._chat_provider.complete(
            system_prompt=system_prompt, user_prompt=user_prompt
        )
        return {"final_answer": final_answer}

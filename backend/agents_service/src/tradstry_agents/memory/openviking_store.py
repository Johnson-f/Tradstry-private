from __future__ import annotations

import json
import math
import os
import re
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Any
from uuid import uuid4

from openviking import AsyncOpenViking

from tradstry_agents.config import Settings
from tradstry_agents.providers import EmbeddingProvider


MEMORY_BUCKETS = ("preferences", "goals", "entities", "events", "patterns", "lessons")


@dataclass
class RetrievedMemory:
    uri: str
    bucket: str
    abstract: str
    content: str
    score: float


class OpenVikingMemoryStore:
    def __init__(self, settings: Settings, embedding_provider: EmbeddingProvider):
        self._settings = settings
        self._embedding_provider = embedding_provider
        self._client: AsyncOpenViking | None = None
        self._base = settings.openviking_data_root

    async def initialize(self) -> None:
        self._base.mkdir(parents=True, exist_ok=True)
        for bucket in ("session", "user", "agent", "queue", "index"):
            (self._base / bucket).mkdir(parents=True, exist_ok=True)
        if self._settings.openrouter_api_key:
            self._ensure_openviking_config()
            self._client = AsyncOpenViking(path=str(self._base))
            await self._client.initialize()

    async def close(self) -> None:
        if self._client is not None:
            await self._client.close()
            self._client = None

    async def append_user_turn(self, *, user_id: str, session_id: str, content: str) -> None:
        if self._client is not None:
            session = self._session(session_id)
            await session.add_message("user", content=content)
            return
        self._append_fallback_session_line(
            user_id=user_id,
            session_id=session_id,
            role="user",
            content=content,
        )

    async def append_assistant_turn(
        self, *, user_id: str, session_id: str, content: str
    ) -> None:
        if self._client is not None:
            session = self._session(session_id)
            await session.add_message("assistant", content=content)
            return
        self._append_fallback_session_line(
            user_id=user_id,
            session_id=session_id,
            role="assistant",
            content=content,
        )

    async def retrieve_context(
        self, *, user_id: str, session_id: str, query: str, limit: int = 4
    ) -> list[RetrievedMemory]:
        docs = await self._load_memory_documents(user_id=user_id)
        if not docs:
            return []

        query_vector = await self._embedding_provider.embed_text(query)
        ranked = []
        for doc in docs:
            score = _cosine_similarity(query_vector, doc["vector"])
            ranked.append(
                RetrievedMemory(
                    uri=doc["uri"],
                    bucket=doc["bucket"],
                    abstract=doc["abstract"],
                    content=doc["content"],
                    score=score,
                )
            )

        ranked.sort(key=lambda item: item.score, reverse=True)
        return ranked[:limit]

    async def promote_memories(
        self, *, user_id: str, request_text: str, response_text: str
    ) -> list[str]:
        candidates = self._build_memory_candidates(request_text)
        stored: list[str] = []
        for candidate in candidates:
            uri = await self._store_memory_doc(
                user_id=user_id,
                bucket=candidate["bucket"],
                title=candidate["title"],
                abstract=candidate["abstract"],
                content=candidate["content"],
                source_text=request_text,
            )
            stored.append(uri)
        return stored

    def _session(self, session_id: str):
        if self._client is None:
            raise RuntimeError("OpenVikingMemoryStore is not initialized")
        return self._client.session(session_id=session_id)

    def _append_fallback_session_line(
        self, *, user_id: str, session_id: str, role: str, content: str
    ) -> None:
        session_root = self._base / "session" / user_id / session_id
        session_root.mkdir(parents=True, exist_ok=True)
        messages_path = session_root / "messages.jsonl"
        payload = {
            "id": f"msg_{uuid4().hex}",
            "role": role,
            "content": content,
            "created_at": datetime.now(UTC).isoformat(),
        }
        with messages_path.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(payload) + "\n")

    async def _load_memory_documents(self, *, user_id: str) -> list[dict[str, Any]]:
        results: list[dict[str, Any]] = []
        user_root = self._base / "user" / user_id
        if not user_root.exists():
            return results

        for detail_path in user_root.glob("*/*/detail.md"):
            bucket = detail_path.parents[1].name
            abstract_path = detail_path.with_name(".abstract.md")
            meta_path = detail_path.with_name(".meta.json")
            if not abstract_path.exists() or not meta_path.exists():
                continue

            meta = json.loads(meta_path.read_text(encoding="utf-8"))
            results.append(
                {
                    "uri": meta["uri"],
                    "bucket": bucket,
                    "abstract": abstract_path.read_text(encoding="utf-8").strip(),
                    "content": detail_path.read_text(encoding="utf-8").strip(),
                    "vector": meta.get("vector", []),
                }
            )
        return results

    async def _store_memory_doc(
        self,
        *,
        user_id: str,
        bucket: str,
        title: str,
        abstract: str,
        content: str,
        source_text: str,
    ) -> str:
        slug = _slugify(title) or uuid4().hex
        doc_root = self._base / "user" / user_id / bucket / slug
        doc_root.mkdir(parents=True, exist_ok=True)
        uri = f"viking://user/{user_id}/{bucket}/{slug}"
        vector = await self._embedding_provider.embed_text(f"{abstract}\n\n{content}")
        meta = {
            "uri": uri,
            "bucket": bucket,
            "title": title,
            "created_at": datetime.now(UTC).isoformat(),
            "updated_at": datetime.now(UTC).isoformat(),
            "source_text": source_text,
            "vector": vector,
        }
        (doc_root / ".meta.json").write_text(json.dumps(meta, indent=2), encoding="utf-8")
        (doc_root / ".abstract.md").write_text(abstract.strip(), encoding="utf-8")
        (doc_root / ".overview.md").write_text(content.strip(), encoding="utf-8")
        (doc_root / "detail.md").write_text(content.strip(), encoding="utf-8")
        return uri

    def _build_memory_candidates(self, request_text: str) -> list[dict[str, str]]:
        text = request_text.strip()
        lowered = text.lower()
        candidates: list[dict[str, str]] = []

        if any(token in lowered for token in ("prefer", "i like", "keep it concise", "be brief")):
            candidates.append(
                {
                    "bucket": "preferences",
                    "title": "answer-style",
                    "abstract": "The user has expressed a response style preference.",
                    "content": text,
                }
            )

        if any(token in lowered for token in ("goal", "focus on", "working on", "i want to improve")):
            candidates.append(
                {
                    "bucket": "goals",
                    "title": "active-trading-goal",
                    "abstract": "The user described an active trading improvement goal.",
                    "content": text,
                }
            )

        if any(token in lowered for token in ("i usually", "i always", "i keep", "my pattern")):
            candidates.append(
                {
                    "bucket": "patterns",
                    "title": "trading-pattern",
                    "abstract": "The user described a recurring trading pattern.",
                    "content": text,
                }
            )

        if any(token in lowered for token in ("today", "this week", "i broke", "i missed")):
            candidates.append(
                {
                    "bucket": "events",
                    "title": "trading-event",
                    "abstract": "The user described a significant recent trading event.",
                    "content": text,
                }
            )

        return candidates

    def _ensure_openviking_config(self) -> None:
        config_path = self._base / "ov.conf"
        if not config_path.exists():
            config = {
                "storage": {"workspace": str(self._base)},
                "embedding": {
                    "dense": {
                        "provider": "openai",
                        "model": self._settings.openrouter_embedding_model,
                        "api_key": self._settings.openrouter_api_key or "local-dev-placeholder",
                        "api_base": _openrouter_api_base(self._settings.openrouter_base_url),
                        "dimension": 1536,
                    }
                },
                "default_search_mode": "fast",
                "default_search_limit": 4,
            }
            config_path.write_text(json.dumps(config, indent=2), encoding="utf-8")
        os.environ.setdefault("OPENVIKING_CONFIG_FILE", str(config_path))


def _slugify(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", value.lower()).strip("-")


def _openrouter_api_base(url: str) -> str:
    if url.endswith("/embeddings"):
        return url[: -len("/embeddings")]
    return url


def _cosine_similarity(left: list[float], right: list[float]) -> float:
    if not left or not right:
        return 0.0
    size = min(len(left), len(right))
    left = left[:size]
    right = right[:size]
    numerator = sum(a * b for a, b in zip(left, right))
    left_norm = math.sqrt(sum(a * a for a in left))
    right_norm = math.sqrt(sum(b * b for b in right))
    if left_norm == 0 or right_norm == 0:
        return 0.0
    return numerator / (left_norm * right_norm)

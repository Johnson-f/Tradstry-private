from __future__ import annotations

import inspect
import json
import math
import os
import re
from dataclasses import dataclass
from datetime import UTC, datetime
from typing import Protocol, TypedDict, cast
from uuid import uuid4

from openviking import AsyncOpenViking  # type: ignore[import-untyped]

from tradstry_agents.config import Settings
from tradstry_agents.providers import EmbeddingProvider


@dataclass(frozen=True)
class RetrievedMemory:
    uri: str
    bucket: str
    abstract: str
    content: str
    score: float


class OpenVikingSession(Protocol):
    def add_message(self, role: str, /, *args: object, **kwargs: object) -> object:
        ...


class OpenVikingClient(Protocol):
    def session(self, *, session_id: str) -> OpenVikingSession:
        ...

    async def initialize(self) -> None:
        ...

    async def close(self) -> None:
        ...


class _StoredMetadata(TypedDict, total=False):
    uri: str
    bucket: str
    title: str
    created_at: str
    updated_at: str
    source_text: str
    vector: list[float]


class _MemoryCandidate(TypedDict):
    bucket: str
    title: str
    abstract: str
    content: str


class _DocumentRecord(TypedDict):
    uri: str
    bucket: str
    abstract: str
    content: str
    vector: list[float]


class _FallbackSessionMessage(TypedDict):
    id: str
    role: str
    content: str
    created_at: str


class OpenVikingMemoryStore:
    def __init__(self, settings: Settings, embedding_provider: EmbeddingProvider):
        self._settings = settings
        self._embedding_provider = embedding_provider
        self._client: OpenVikingClient | None = None
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
            await self._append_session_message(session, "user", content)
            return
        self._append_fallback_session_line(
            user_id=user_id,
            session_id=session_id,
            role="user",
            content=content,
        )

    async def append_assistant_turn(self, *, user_id: str, session_id: str, content: str) -> None:
        if self._client is not None:
            session = self._session(session_id)
            await self._append_session_message(session, "assistant", content)
            return
        self._append_fallback_session_line(
            user_id=user_id,
            session_id=session_id,
            role="assistant",
            content=content,
        )

    async def _append_session_message(self, session: OpenVikingSession, role: str, content: str) -> None:
        call_result: object
        try:
            call_result = session.add_message(role, content=content)
        except TypeError:
            call_result = session.add_message(role, content)
        await _await_if_needed(call_result)

    async def retrieve_context(
        self, *, user_id: str, session_id: str, query: str, limit: int = 4
    ) -> list[RetrievedMemory]:
        del session_id
        documents = await self._load_memory_documents(user_id=user_id)
        if not documents:
            return []

        query_vector = await self._embedding_provider.embed_text(query)
        ranked: list[RetrievedMemory] = []
        for document in documents:
            score = _cosine_similarity(query_vector, document["vector"])
            ranked.append(
                RetrievedMemory(
                    uri=document["uri"],
                    bucket=document["bucket"],
                    abstract=document["abstract"],
                    content=document["content"],
                    score=score,
                )
            )

        ranked.sort(key=lambda item: item.score, reverse=True)
        return ranked[:limit]

    async def promote_memories(
        self, *, user_id: str, request_text: str, response_text: str
    ) -> list[str]:
        if not response_text.strip():
            return []

        candidates = self._build_memory_candidates(request_text=request_text)
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

    def _session(self, session_id: str) -> OpenVikingSession:
        if self._client is None:
            raise RuntimeError("OpenVikingMemoryStore is not initialized")
        return self._client.session(session_id=session_id)

    def _append_fallback_session_line(
        self, *, user_id: str, session_id: str, role: str, content: str
    ) -> None:
        session_root = self._base / "session" / user_id / session_id
        session_root.mkdir(parents=True, exist_ok=True)
        messages_path = session_root / "messages.jsonl"
        payload: _FallbackSessionMessage = {
            "id": f"msg_{uuid4().hex}",
            "role": role,
            "content": content,
            "created_at": datetime.now(UTC).isoformat(),
        }
        with messages_path.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(payload) + "\n")

    async def _load_memory_documents(self, *, user_id: str) -> list[_DocumentRecord]:
        results: list[_DocumentRecord] = []
        user_root = self._base / "user" / user_id
        if not user_root.exists():
            return results

        for detail_path in user_root.glob("*/*/detail.md"):
            bucket = detail_path.parents[1].name
            abstract_path = detail_path.with_name(".abstract.md")
            meta_path = detail_path.with_name(".meta.json")
            if not abstract_path.exists() or not meta_path.exists():
                continue

            metadata = _coerce_dict(meta_path.read_text(encoding="utf-8"))
            if not metadata:
                continue

            uri = metadata.get("uri")
            if not isinstance(uri, str):
                continue

            abstract = abstract_path.read_text(encoding="utf-8").strip()
            if not abstract:
                continue

            vector = _coerce_vector(metadata.get("vector"))
            if not vector:
                continue

            content = detail_path.read_text(encoding="utf-8").strip()
            results.append(
                _DocumentRecord(
                    uri=uri,
                    bucket=bucket,
                    abstract=abstract,
                    content=content,
                    vector=vector,
                )
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
        metadata: _StoredMetadata = {
            "uri": uri,
            "bucket": bucket,
            "title": title,
            "created_at": datetime.now(UTC).isoformat(),
            "updated_at": datetime.now(UTC).isoformat(),
            "source_text": source_text,
            "vector": vector,
        }
        (doc_root / ".meta.json").write_text(json.dumps(metadata, indent=2), encoding="utf-8")
        (doc_root / ".abstract.md").write_text(abstract.strip(), encoding="utf-8")
        (doc_root / ".overview.md").write_text(content.strip(), encoding="utf-8")
        (doc_root / "detail.md").write_text(content.strip(), encoding="utf-8")
        return uri

    def _build_memory_candidates(self, *, request_text: str) -> list[_MemoryCandidate]:
        text = request_text.strip()
        lowered = text.lower()
        candidates: list[_MemoryCandidate] = []

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


async def _await_if_needed(result: object) -> None:
    if inspect.isawaitable(result):
        await result


def _coerce_dict(raw_payload: str) -> _StoredMetadata:
    try:
        parsed = json.loads(raw_payload)
    except json.JSONDecodeError:
        return {}
    if not isinstance(parsed, dict):
        return {}
    return cast(_StoredMetadata, parsed)


def _coerce_vector(value: object) -> list[float]:
    if not isinstance(value, list):
        return []

    vectors: list[float] = []
    for item in value:
        if isinstance(item, int | float):
            vectors.append(float(item))
    return vectors


def _slugify(value: str) -> str:
    lowered = value.lower().strip()
    return re.sub(r"[^a-z0-9]+", "-", lowered).strip("-")


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

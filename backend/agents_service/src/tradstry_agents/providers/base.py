from __future__ import annotations

from typing import Protocol


class ChatProvider(Protocol):
    async def complete(self, *, system_prompt: str, user_prompt: str) -> str:
        ...


class EmbeddingProvider(Protocol):
    async def embed_text(self, text: str) -> list[float]:
        ...

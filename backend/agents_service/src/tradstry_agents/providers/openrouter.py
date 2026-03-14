from __future__ import annotations

import hashlib

import httpx

from tradstry_agents.config import Settings


class OpenRouterEmbeddingProvider:
    def __init__(self, settings: Settings):
        self._settings = settings

    async def embed_text(self, text: str) -> list[float]:
        if not self._settings.openrouter_api_key:
            return self._fallback_embedding(text)

        payload = {
            "model": self._settings.openrouter_embedding_model,
            "input": text,
        }

        async with httpx.AsyncClient(timeout=self._settings.response_timeout_seconds) as client:
            response = await client.post(
                self._settings.openrouter_base_url,
                headers={
                    "Authorization": f"Bearer {self._settings.openrouter_api_key}",
                    "Content-Type": "application/json",
                },
                json=payload,
            )
            response.raise_for_status()
            body = response.json()

        try:
            return list(body["data"][0]["embedding"])
        except (KeyError, IndexError, TypeError):
            return self._fallback_embedding(text)

    def _fallback_embedding(self, text: str) -> list[float]:
        digest = hashlib.sha256(text.encode("utf-8")).digest()
        # Small deterministic vector for local ranking when OpenRouter is absent.
        return [byte / 255.0 for byte in digest[:32]]

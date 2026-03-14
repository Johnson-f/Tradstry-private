from __future__ import annotations

from textwrap import shorten

import httpx
from pydantic import BaseModel

from tradstry_agents.config import Settings


class _GroqCompletionMessage(BaseModel):
    content: str


class _GroqChoice(BaseModel):
    message: _GroqCompletionMessage


class _GroqCompletionResponse(BaseModel):
    choices: list[_GroqChoice]


class GroqChatProvider:
    def __init__(self, settings: Settings):
        self._settings = settings

    async def complete(self, *, system_prompt: str, user_prompt: str) -> str:
        if not self._settings.groq_api_key:
            return self._fallback_response(user_prompt)

        payload = {
            "model": self._settings.groq_model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt},
            ],
            "temperature": 0.2,
        }

        async with httpx.AsyncClient(timeout=self._settings.response_timeout_seconds) as client:
            response = await client.post(
                self._settings.groq_base_url,
                headers={
                    "Authorization": f"Bearer {self._settings.groq_api_key}",
                    "Content-Type": "application/json",
                },
                json=payload,
            )
            response.raise_for_status()
            body = response.json()

        try:
            parsed = _GroqCompletionResponse.model_validate(body)
            return parsed.choices[0].message.content.strip()
        except (ValueError, TypeError, IndexError):
            return self._fallback_response(user_prompt)

    def _fallback_response(self, user_prompt: str) -> str:
        preview = shorten(user_prompt.replace("\n", " "), width=320, placeholder="...")
        return (
            "Groq is not configured, so this is a deterministic fallback answer.\n\n"
            f"Request summary: {preview}"
        )

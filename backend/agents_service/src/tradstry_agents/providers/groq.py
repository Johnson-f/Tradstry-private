from __future__ import annotations

import logging
from textwrap import shorten

import httpx
from pydantic import BaseModel

from tradstry_agents.config import Settings

logger = logging.getLogger(__name__)


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
            return self._fallback_response(user_prompt, reason="not_configured")

        payload = {
            "model": self._settings.groq_model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt},
            ],
            "temperature": 0.2,
        }

        try:
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
        except httpx.HTTPStatusError as exc:
            logger.warning(
                "Groq chat completion returned %s for %s",
                exc.response.status_code,
                self._settings.groq_base_url,
            )
            return self._fallback_response(user_prompt, reason="request_failed")
        except httpx.RequestError as exc:
            logger.warning("Groq chat completion request failed: %s", exc)
            return self._fallback_response(user_prompt, reason="request_failed")

        try:
            parsed = _GroqCompletionResponse.model_validate(body)
            return parsed.choices[0].message.content.strip()
        except (ValueError, TypeError, IndexError):
            logger.warning("Groq chat completion response could not be parsed")
            return self._fallback_response(user_prompt, reason="invalid_response")

    def _fallback_response(
        self,
        user_prompt: str,
        *,
        reason: str,
    ) -> str:
        preview = shorten(user_prompt.replace("\n", " "), width=320, placeholder="...")
        if reason == "not_configured":
            intro = "Groq is not configured, so this is a deterministic fallback answer."
        elif reason == "request_failed":
            intro = "Groq is temporarily unavailable, so this is a deterministic fallback answer."
        else:
            intro = "Groq returned an invalid response, so this is a deterministic fallback answer."
        return (
            f"{intro}\n\n"
            f"Request summary: {preview}"
        )

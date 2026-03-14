from __future__ import annotations

from functools import lru_cache
from pathlib import Path

from pydantic import Field, field_validator
from pydantic_settings import BaseSettings, SettingsConfigDict


def _service_root() -> Path:
    return Path(__file__).resolve().parents[3]


class Settings(BaseSettings):
    model_config = SettingsConfigDict(
        env_file_encoding="utf-8",
        extra="ignore",
    )

    host: str = Field(default="0.0.0.0", validation_alias="AGENTS_SERVICE_HOST")
    port: int = Field(default=8091, validation_alias="AGENTS_SERVICE_PORT")
    websocket_path: str = Field(
        default="/ws/v1/agents",
        validation_alias="AGENTS_SERVICE_WEBSOCKET_PATH",
    )
    prompt_root: Path = Field(
        default_factory=lambda: _service_root() / "src" / "tradstry_agents" / "prompts",
        validation_alias="AGENTS_PROMPT_ROOT",
    )
    groq_api_key: str | None = Field(default=None, validation_alias="GROQ_API_KEY")
    groq_model: str = Field(default="llama-3.3-70b-versatile", validation_alias="GROQ_MODEL")
    groq_base_url: str = Field(
        default="https://api.groq.com/openai/v1/chat/completions",
        validation_alias="GROQ_BASE_URL",
    )
    openrouter_api_key: str | None = Field(default=None, validation_alias="OPENROUTER_API_KEY")
    openrouter_embedding_model: str = Field(
        default="text-embedding-3-small",
        validation_alias="OPENROUTER_EMBEDDING_MODEL",
    )
    openrouter_base_url: str = Field(
        default="https://openrouter.ai/api/v1/embeddings",
        validation_alias="OPENROUTER_BASE_URL",
    )
    openviking_data_root: Path = Field(
        default_factory=lambda: _service_root() / "data" / "openviking",
        validation_alias="OPENVIKING_DATA_ROOT",
    )
    response_timeout_seconds: float = Field(
        default=60.0,
        validation_alias="AGENTS_RESPONSE_TIMEOUT_SECONDS",
    )
    tool_timeout_seconds: float = Field(
        default=20.0,
        validation_alias="AGENTS_TOOL_TIMEOUT_SECONDS",
    )
    heartbeat_interval_seconds: float = Field(
        default=15.0,
        validation_alias="AGENTS_HEARTBEAT_INTERVAL_SECONDS",
    )

    @field_validator("prompt_root", "openviking_data_root", mode="before")
    @classmethod
    def _coerce_path(cls, value: str | Path) -> Path:
        return Path(value).expanduser()


@lru_cache(maxsize=1)
def load_settings() -> Settings:
    return Settings(_env_file=_service_root() / ".env")

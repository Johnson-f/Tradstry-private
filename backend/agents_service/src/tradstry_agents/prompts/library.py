from __future__ import annotations

from functools import lru_cache

from tradstry_agents.config import Settings


class PromptLibrary:
    def __init__(self, settings: Settings):
        self._root = settings.prompt_root

    def compose_system_prompt(self, route: str) -> str:
        sections = [
            self._read("shared/base_system.txt"),
            self._read("shared/tool_rules.txt"),
            self._read("shared/tool_examples.txt"),
            self._read(f"routes/{route}.txt"),
        ]
        return "\n\n".join(section.strip() for section in sections if section.strip())

    @lru_cache(maxsize=16)
    def _read(self, relative_path: str) -> str:
        return (self._root / relative_path).read_text(encoding="utf-8")

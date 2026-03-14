## This folder will contain python code that will contain the logic for building agents with LangGraph and OpenViking 

## Developer setup

### Install

```bash
cd backend/agents_service
uv sync --group dev
```

### Mypy checks

- Quick (baseline): `uv run mypy --config-file pyproject.toml src`
- Strict target: `uv run mypy --config-file mypy-strict.ini src`

### Additional checks

- `python -m compileall backend/agents_service/src/tradstry_agents`

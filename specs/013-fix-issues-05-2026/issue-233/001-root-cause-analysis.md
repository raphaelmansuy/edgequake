# Issue #233 — Root Cause Analysis

**GitHub:** [#233](https://github.com/raphaelmansuy/edgequake/issues/233)

## Symptom (fact)

Workspace creation UI forces LLM, embedding, and vision model selection even when ECS task definition already sets server defaults.

## 5 WHY

| # | Why | Evidence |
|---|-----|----------|
| 1 | Why complex UI? | Create dialog always renders three required model selectors |
| 2 | Why required? | Create button `disabled` until all three selections set |
| 3 | Why not use env defaults? | UI never calls `/api/v1/models` to detect `default_*` fields |
| 4 | Why server has defaults? | `models.toml` / `EDGEQUAKE_DEFAULT_*` env vars populate API defaults |
| 5 | Why ECS users care? | Task definition configures models once; per-workspace override is rare |

## Fix summary

`WorkspaceCreateModelSection`: collapse model config when `fetchModelsConfig()` reports defaults; omit model fields from create payload (server inherits).

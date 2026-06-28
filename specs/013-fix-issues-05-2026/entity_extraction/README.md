# Entity extraction strict mode (SPEC-013 extension)

Workspace-level control for whether entity types are **strictly limited** to the configured list (unknown types remapped to `OTHER`) or **permissive** (LLM may emit additional type labels; no forced `OTHER` catch-all).

## Documents

| File | Purpose |
|------|---------|
| [001-requirements.md](./001-requirements.md) | Functional requirements and edge cases |
| [002-technical-design.md](./002-technical-design.md) | API, metadata, pipeline, UI |
| [003-test-plan.md](./003-test-plan.md) | Unit, API, Playwright proof |

## Metadata key

- `entity_types_strict`: `boolean` in `workspaces.metadata`
- **Default when absent:** `true` (preserves post–#217 behavior)

## Related issues

- SPEC-085: per-workspace `entity_types`
- GitHub #217: server-side type enforcement (strict mode)
- GitHub #216: API/UI edit `entity_types`

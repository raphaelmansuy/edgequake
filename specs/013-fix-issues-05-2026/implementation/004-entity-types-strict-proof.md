# Proof — Entity types strict limit checkbox

## Summary

Implemented `entity_types_strict` workspace setting and UI checkbox **“Limit extraction to listed types (classify others as OTHER)”**.

| Mode | Prompt | Post-parse |
|------|--------|------------|
| **Strict (default)** | MUST use only listed types; OTHER fallback | Unknown → OTHER / CONCEPT / first |
| **Permissive (unchecked)** | Prefer listed types; no OTHER catch-all | Unknown types kept (normalized) |

## Automated proof (passed)

### Pipeline unit tests

```bash
cargo test -p edgequake-pipeline --lib entity_type -- --nocapture
```

Log: [evidence/rust-pipeline-entity-strict.log](./evidence/rust-pipeline-entity-strict.log) — **9/9 passed**

### API E2E (PostgreSQL, in-process)

```bash
export DATABASE_URL=postgres://edgequake:edgequake_secret@localhost:5432/edgequake
cargo test -p edgequake-api --features postgres --test e2e_spec013_github_issues spec013_entity_types_strict -- --nocapture
```

Log: [evidence/api-entity-types-strict.log](./evidence/api-entity-types-strict.log) — **1/1 passed**

### Playwright API (live backend, port 8086, auth off)

```bash
make backend-bg BACKEND_PORT=8086 DEV_AUTH_ENABLED=false
cd edgequake_webui && E2E_BACKEND_URL=http://localhost:8086 pnpm exec playwright test entity-types-strict-limit.spec.ts -g "API persists"
```

**Result:** passed — `entity_types_strict` false/true round-trip on REST API.

## UI screenshots (Playwright — 2/2 passed)

```bash
make backend-bg BACKEND_PORT=8086 DEV_AUTH_ENABLED=false
cd edgequake_webui
E2E_BACKEND_URL=http://localhost:8086 PLAYWRIGHT_BASE_URL=http://localhost:3001 \
  pnpm exec playwright test entity-types-strict-limit.spec.ts
```

Use port **3001** for EdgeQuake WebUI when port 3000 is another application.

- [screenshots/entity-types-strict-checked.png](./screenshots/entity-types-strict-checked.png)
- [screenshots/entity-types-strict-unchecked.png](./screenshots/entity-types-strict-unchecked.png)

## Files touched

- `edgequake-pipeline/src/prompts/entity_type_policy.rs` — `EntityExtractionSchema`, enforcement, prompt sections
- `edgequake-api/src/processor/workspace_resolver.rs` — schema into `LLMExtractor`
- `edgequake-core` / `edgequake-api` — metadata + API DTOs
- `edgequake_webui` — `EntityTypeSelector` checkbox, workspace page save/display
- `e2e/entity-types-strict-limit.spec.ts`

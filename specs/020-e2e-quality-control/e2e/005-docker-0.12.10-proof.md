# SPEC-020 — Docker GHCR v0.12.10 E2E Proof

**Date:** 2026-06-08  
**Images:** `ghcr.io/raphaelmansuy/edgequake:0.12.10` (+ frontend, postgres)  
**CD:** [Release workflow run](https://github.com/raphaelmansuy/edgequake/actions/runs/27140508450) — success  
**PR:** [#249](https://github.com/raphaelmansuy/edgequake/pull/249)

## Stack

```bash
EDGEQUAKE_VERSION=0.12.10 EDGEQUAKE_PORT=18080 FRONTEND_PORT=13000 \
  EDGEQUAKE_LLM_PROVIDER=mock EDGEQUAKE_API_URL=http://localhost:18080 \
  docker compose -f docker-compose.quickstart.yml up -d
```

| Check | Result |
|-------|--------|
| API health | `version: 0.12.10`, `llm_provider_name: mock` |
| Ollama sync timeout | 600s for workspace `ollama` (was 408 @ 120s on v0.12.9) |

## Playwright SPEC-020

```bash
E2E_LIVE_STACK=1 PLAYWRIGHT_BASE_URL=http://localhost:13000 \
  EQ_BACKEND_URL=http://localhost:18080 SPEC020_STRICT_MIGRATION=1 \
  bunx playwright test e2e/spec020-quality-control.spec.ts --project=audit --workers=1
```

| Result | Count |
|--------|-------|
| **Passed** | **24** |
| Failed | 0 |
| Skipped | 0 |
| Duration | ~1.9 min |

**Ollama proofs (previously failing on v0.12.8/v0.12.9):**

| Test | v0.12.9 | v0.12.10 |
|------|---------|----------|
| 10 — live Ollama query | 408 timeout | ✅ 33.4s |
| 19 — entity extraction | 408 timeout | ✅ 28.4s |
| 22 — workspace stats | 408 timeout | ✅ 36.1s |

## Grade: **A+** — full Docker GHCR E2E (mock + live Ollama)

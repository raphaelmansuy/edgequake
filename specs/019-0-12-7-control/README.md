# SPEC-019 — v0.12.7 Control (Option 1 Install)

**Status:** ✅ PASS  
**Date:** 2026-06-07  
**Release:** [v0.12.7](https://github.com/raphaelmansuy/edgequake/releases/tag/v0.12.7)  
**Scope:** Verify README **Option 1 — One Command (Docker)** works E2E against published GHCR images.

## What Option 1 means

From [README Quick Start](https://github.com/raphaelmansuy/edgequake#-option-1--one-command-docker-30s-no-build-required):

```bash
curl -fsSL https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main/quickstart.sh | sh
```

Equivalent headless path (same compose asset, CI-friendly):

```bash
curl -fsSL https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main/docker-compose.quickstart.yml \
  -o docker-compose.quickstart.yml
EDGEQUAKE_VERSION=0.12.7 docker compose -f docker-compose.quickstart.yml up -d
```

## Proof index

| # | Artifact | Result |
|---|----------|--------|
| 001 | [Option 1 install proof](e2e/001-option1-install-proof.md) | ✅ PASS |
| 002 | [Upload + query proof](e2e/002-upload-query-proof.md) | ✅ PASS |
| — | [Health JSON](e2e/002-health-response.json) | `version: 0.12.7`, `status: healthy` |
| — | [Upload JSON](e2e/009-upload-response.json) | 12 entities, processed |
| — | [Query JSON](e2e/010-query-response.json) | 27 sources, Sarah Chen answer |
| 003–005 | Image pins | `ghcr.io/.../edgequake{,-frontend,-postgres}:0.12.7` |
| 008 | [Compose ps](e2e/008-compose-ps.txt) | 3/3 healthy |
| — | [Install log](e2e/001-install-run.log) | Pull + start transcript |
| — | [Upload/query log](e2e/011-upload-query-run.log) | API + UI transcript |
| — | Screenshots | `e2e/screenshots/01`–`07-*.png` |

## Reproduce

```bash
# Full chain: install + upload + query + screenshots
bash specs/019-0-12-7-control/e2e/run_option1_install_proof.sh

# Upload/query only (stack already running):
bash specs/019-0-12-7-control/e2e/run_upload_query_proof.sh
cd edgequake_webui
EQ_BACKEND_URL=http://127.0.0.1:18080 PLAYWRIGHT_BASE_URL=http://127.0.0.1:13000 \
  bunx playwright test e2e/spec019-option1-upload-query.spec.ts --project=chromium
```

Default proof ports: API `18080`, UI `13000` (avoids local dev collisions).  
Requires **Ollama** on the host for default provider path.

## Verdict

| Check | Status |
|-------|--------|
| GitHub raw compose downloads | ✅ |
| GHCR images pull (`0.12.7`) | ✅ |
| API `/health` → `0.12.7` | ✅ |
| UI renders (dashboard, documents, query) | ✅ |
| Swagger UI reachable | ✅ |
| `quickstart.sh` downloadable + valid shell | ✅ |
| Document upload (sync, Ollama) | ✅ 12 entities |
| Hybrid query with sources | ✅ 27 sources |
| UI query answer (Playwright) | ✅ Sarah Chen profile |

**GO** — Option 1 install path is production-verified for v0.12.7 including ingest → query.

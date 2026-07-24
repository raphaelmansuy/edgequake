# SPEC-086 — Surface Playbooks

> OCP: add surface recipes here; do not copy into findings.  
> Prefer Makefile / existing 068 commands.

---

## 1. API / Rust

```bash
cd edgequake

# 068 must remain green
cargo test -p edgequake-api --test contract_068_text_ingest_progress

# Wave 1: staging list/track/activity visibility
cargo test -p edgequake-api --test contract_086_ingestion_visibility
cargo test -p edgequake-api --lib services::pipeline_ws_bridge

cargo test -p edgequake-api --lib services::document_metadata_scan
cargo test -p edgequake-api --lib handlers::ingestion
cargo fmt --check -p edgequake-api
```

Manual admit smoke:

```bash
# Health
curl -s http://localhost:8080/health | python3 -m json.tool

# Text/MD admit (auth headers as per local .env)
# Expect 202 + task_id insert-*
# Then: GET /api/v1/ingestion/{task_id}/progress  → 200 with staging
# Then: GET /api/v1/documents                   → in-flight MD visible (Wave 1)
```

---

## 2. WebUI

```bash
cd edgequake_webui

pnpm exec vitest run \
  src/hooks/__tests__/use-ingestion-progress-068.test.ts \
  src/hooks/__tests__/use-ingestion-progress-086.test.ts \
  src/lib/pipeline/__tests__/merge-ingestion-progress.test.ts \
  src/lib/pipeline/__tests__/ingestion-run-view-086.test.ts \
  src/lib/upload/__tests__/perform-file-upload.test.ts \
  src/lib/upload/__tests__/file-kind.test.ts \
  src/lib/upload/__tests__/progress-track-id.test.ts \
  src/components/documents/__tests__/ingestion-run-card-086.test.ts
```

---

## 3. Playwright e2e

```bash
cd edgequake_webui

# Prefer localhost (Next.js 16 blocks cross-origin _next from 127.0.0.1)
PLAYWRIGHT_BASE_URL=http://localhost:3010 PLAYWRIGHT_SKIP_STACK_CHECK=1 \
  pnpm exec playwright test e2e/spec068-text-ingest-progress.spec.ts --project=chromium

# Wave 4:
PLAYWRIGHT_BASE_URL=http://localhost:3010 PLAYWRIGHT_SKIP_STACK_CHECK=1 \
  pnpm exec playwright test e2e/spec086-ingestion-ux.spec.ts --project=chromium
```

Stack:

```bash
# from repo root
make status
# or make dev-bg && make status
```

---

## 4. Quality / density (Wave 3)

```bash
python3 scripts/ingestion_density_gate.py \
  --fixture specs/086-improve-ingestion-ux/fixtures/density-golden-pair-v1.json
```

---

## 5. Docs SSOT (read-only)

| Doc | When |
|-----|------|
| [`docs/deep-dives/pipeline-progress.md`](../../docs/deep-dives/pipeline-progress.md) | Progress channels |
| [`docs/ingestion-cancel-and-fairness.md`](../../docs/ingestion-cancel-and-fairness.md) | Cancel / queue |
| [`docs/OBSERVABILITY.md`](../../docs/OBSERVABILITY.md) | Metrics |

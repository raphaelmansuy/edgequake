# Spec 018 — Observability & OTEL

**Date:** 2026-06-05 (post-implementation)  
**Status:** Implemented — see [015-brutal-post-implementation.md](./015-brutal-post-implementation.md)

---

## Verdict (current)

| Pillar | Grade | Evidence |
|--------|-------|----------|
| **Correlation** | A | `X-Request-ID` + W3C `traceparent` (echo/synthesize/inject); WebUI client |
| **Structured logs** | A+ | `EDGEQUAKE_LOG_FORMAT=json`; levelled `ErrorEvent`; domain error/warn |
| **Traces** | A | `http_request` + pipeline + query spans; Jaeger overlay |
| **Error context** | A+ | API `details.diagnostics` with category, retryable, auth reason, phase |
| **Metrics** | A | HTTP, query, LLM, document, storage, pipeline, db pool gauges |
| **OTLP export** | A- | `docker-compose.observability.yml` + `--profile observability` |
| **Audit** | B+ | Auth, query, upload, PDF, delete, workspace CRUD |

Run proofs: [`e2e/run_observability_proof.sh`](./e2e/run_observability_proof.sh)

---

## P0 gaps — resolution status

| ID | Gap | Status |
|----|-----|--------|
| OBS-P0-001 | No OTEL exporter | **FIXED (opt-in)** — `edgequake-observability/otel` + env |
| OBS-P0-002 | request_id not on spans | **FIXED** — `with_http_span`, task-local |
| OBS-P0-003 | WebUI no correlation headers | **FIXED** — `client.ts` X-Request-ID + traceparent |
| OBS-P0-004 | `/metrics` placeholder | **FIXED** — Prometheus recorder + describe_* |
| OBS-P0-005 | Audit unwired | **FIXED (partial)** — query, auth, upload, workspace |
| OBS-P0-006 | Docs vs JSON logging | **FIXED** — `init_observability` + `EDGEQUAKE_LOG_FORMAT` |

---

## Key crates

| Crate | Role |
|-------|------|
| `edgequake-observability` | Subscriber, metrics, `ErrorEvent`, HTTP spans, correlation |
| `edgequake-api` | `observability_middleware`, `ApiError` diagnostics, audit helpers |

---

## Production quick start

```bash
# Build with OTLP
docker compose -f edgequake/docker/docker-compose.yml build \
  --build-arg ENABLE_OTEL=true

# Runtime
export EDGEQUAKE_LOG_FORMAT=json
export OTEL_EXPORTER_OTLP_ENDPOINT=http://collector:4317
export RUST_LOG=edgequake_api=info,edgequake_storage=warn
```

Operator guide: [`docs/OBSERVABILITY.md`](../../docs/OBSERVABILITY.md)

---

## Document index (audits — historical)

Subfolder audits describe **pre-implementation** state. Use this README + `015-brutal-post-implementation.md` as source of truth.

| Folder | Component |
|--------|-----------|
| [003-edgequake-api](./003-edgequake-api/001-audit.md) | HTTP middleware |
| [005-edgequake-pipeline](./005-edgequake-pipeline/001-audit.md) | Ingestion |
| [013-edgequake-webui](./013-edgequake-webui/001-audit.md) | Client (updated in code) |

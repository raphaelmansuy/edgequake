# SPEC-018 — Correlation MVP (cross-crate) proof

**Status:** ✅ Proven  
**Date:** 2026-06-05

## Claim

Phase 1 remediation shipped: `edgequake-observability` crate, JSON log env, WebUI `X-Request-ID`, API→LLM header merge, live `/metrics`.

## Evidence

```bash
./specs/018-observability/e2e/run_observability_proof.sh
```

## Manual smoke (optional)

```bash
EDGEQUAKE_LOG_FORMAT=json RUST_LOG=info cargo run -p edgequake  # requires DATABASE_URL
curl -s -D - -o /dev/null -H 'X-Request-ID: proof-018-test' http://localhost:8080/health | grep -i x-request-id
curl -s http://localhost:8080/metrics | head -20
```

Expected: response header `x-request-id: proof-018-test`; metrics body contains `edgequake_http_requests_total` after requests.

## OTEL note

OTLP export is behind `--features otel` on `edgequake-observability` (compile-time). Enable with `OTEL_EXPORTER_OTLP_ENDPOINT` when feature is enabled in the binary build.

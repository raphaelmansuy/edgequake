# SPEC-018 — edgequake-llm header propagation proof

**Status:** ✅ Proven (API merge)  
**Date:** 2026-06-05

## Claim

Inbound `traceparent` / `x-request-id` harvested by middleware reach `LlmResolutionRequest.extra_headers` without manual JSON body.

## Evidence

```bash
rg 'harvest_propagation_headers' edgequake/crates/edgequake-api/src/observability_middleware.rs
rg 'merge_with' edgequake/crates/edgequake-api/src/handlers/query/query_execute.rs
```

See also [HEADER_PROPAGATION](../../edgequake-llm-update/HEADER_PROPAGATION.md).

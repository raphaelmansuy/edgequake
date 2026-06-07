# SPEC-018 — API request ID + observability middleware proof

**Status:** ✅ Proven  
**Date:** 2026-06-05

## Claim

Unified `observability_middleware` honors inbound `x-request-id`, records Prometheus HTTP metrics, and returns the same ID on responses.

## Evidence

```bash
cargo test -p edgequake-api --test observability_proof
cargo test -p edgequake-api --test integration_tests test_request_id_header_added
```

| Check | Result |
|-------|--------|
| `spec018_honors_inbound_request_id_header` | ✅ |
| `spec018_resolve_request_id_unit` | ✅ |
| Integration health response includes `x-request-id` | ✅ |

## Code (law)

- `edgequake/crates/edgequake-api/src/observability_middleware.rs`
- `edgequake/crates/edgequake-api/src/server.rs` (single middleware layer)
- `edgequake/crates/edgequake-api/src/handlers/metrics.rs` (live Prometheus render)

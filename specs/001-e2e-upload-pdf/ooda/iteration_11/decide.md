# OODA-11 Decide: Timeout Enforcement Implementation

## Decision
Create `e2e_timeout_enforcement.rs` with 8 timeout-guarded tests covering critical paths.

## Timeout Budget

| Category | Timeout | Rationale |
|----------|---------|-----------|
| Health check | 5s | No I/O, just config |
| Tenant creation | 5s | In-memory only |
| Small doc upload | 10s | Single chunk, mock pipeline |
| Medium doc upload | 30s | Multiple chunks, mock extraction |
| Large doc upload | 30s | Many chunks, mock extraction |
| Full pipeline | 30s | Upload + graph check + document retrieval |
| Query after ingestion | 30s | Upload + graph traversal + mock LLM |
| Sequential uploads | 30s | 3 uploads, verifies no accumulating latency |

## Test Coverage Map

```
Health Check (5s)
  └─ GET /health → 200 OK, status: "healthy"

Tenant Creation (5s)
  └─ POST /api/v1/tenants → 201 Created

Small Upload (10s)
  └─ POST /api/v1/documents → 201 Created, status: "processed"

Medium Upload (30s)
  └─ POST /api/v1/documents → 201 Created

Large Upload (30s)
  └─ POST /api/v1/documents → 201 Created

Full Pipeline (30s)
  ├─ POST /api/v1/documents → 201 Created
  ├─ GET /api/v1/documents/{id} → 200 OK
  └─ GET /api/v1/graph → 200 OK

Query After Ingestion (30s)
  ├─ POST /api/v1/documents → 201 Created
  └─ POST /api/v1/query → 200 OK, answer present

Sequential Uploads (30s)
  ├─ POST /api/v1/documents (small) → 201
  ├─ POST /api/v1/documents (medium) → 201
  └─ POST /api/v1/documents (large) → 201
```

## Implementation Pattern
```rust
async fn with_timeout<F, T>(duration: Duration, future: F) -> Result<T, String>
```

## Files to Create
1. `edgequake/crates/edgequake-api/tests/e2e_timeout_enforcement.rs`

## Risk Assessment
- **Zero risk**: New file only, no modifications to existing tests
- **Regression**: None possible — additive change only

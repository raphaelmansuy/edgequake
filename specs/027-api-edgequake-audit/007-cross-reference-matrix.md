# Cross-Reference Matrix

**Spec:** 027-api-edgequake-audit  
**Re-assessed:** 2026-06-28 (phase 52 — AUTH scope closed)  
**Legend:** FIXED | PARTIAL | OPEN | OPT-IN (fixed only when env set)

---

## Master Finding Table

| ID | P | Lens | Finding | Primary File(s) | IMP | **Status** |
|----|---|------|---------|-----------------|-----|------------|
| OAS-001 | P1 | OpenAPI | Dual registry drift | `route_registry.rs` | IMP-011 | **FIXED** (bidirectional CI phase 13) |
| OAS-002 | P1 | OpenAPI | `/api/models` path bug | `handlers/models.rs` | IMP-012 | **FIXED** |
| OAS-003 | P1 | OpenAPI | Document CRUD missing | `openapi.rs` | IMP-010 | **FIXED** |
| OAS-004 | P2 | OpenAPI | Admin/injection invisible | `openapi.rs` | IMP-010 | **FIXED** |
| OAS-005 | P2 | OpenAPI | SSE/stream types absent | stream handlers | IMP-013 | **FIXED** |
| OAS-006 | P2 | OpenAPI | Security schemes incomplete | `openapi_security.rs` | IMP-014 | **FIXED** |
| OAS-007 | P3 | OpenAPI | Phantom routes | `route_registry.rs` | IMP-011 | **FIXED** (reverse CI phase 13) |
| OAS-008 | P3 | OpenAPI | WebSocket gap | `openapi_enrichment.rs` | IMP-011 | **FIXED** (101 + x-extension + AsyncAPI sidecar phase 14) |
| OAS-010 | P2 | OpenAPI | Handler annotation drift | `openapi_annotation_sync.rs` | IMP-011 | **FIXED** (phase 14) |
| OAS-009 | P2 | OpenAPI | No frontend codegen | `edgequake_webui/scripts/codegen-openapi.sh` | IMP-011 | **FIXED** (phase 15) |
| OAS-010 | P3 | OpenAPI | utoipa version drift | `Cargo.toml` | IMP-011 | **FIXED** — `=5.4.0` |
| OAS-011 | P3 | OpenAPI | PATCH user missing | `user_management.rs` | IMP-011 | **FIXED** |
| REST-001 | P2 | REST | RPC POST actions | `routes.rs`, `v2/jobs/` | IMP-025 | **DONE (L4 v2)**; v1 RPC open |
| REST-002 | P1 | REST | DELETE /documents all | `bulk.rs` | IMP-018 | **OPT-IN** |
| REST-003 | P2 | REST | Three entity ID spaces | `entity_graph_lookup.rs` | IMP-026 | **PARTIAL** (lookup SSOT + OAS docs) |
| REST-007 | P2 | REST | Pagination inconsistency | `list_pagination.rs` | IMP-020 | **FIXED** |
| REST-009 | P2 | REST | share_url wrong path | `share_paths.rs` | IMP-027 | **FIXED** |
| REST-010 | P3 | REST | Not RFC 7807 | `problem_details.rs` | IMP-028 | **FIXED** (hybrid body) |
| REST-024 | P3 | REST | v1 RPC no Sunset/Link | `v1_rpc_migration.rs` | IMP-025 | **FIXED** (phase 22) |
| REST-025 | P3 | REST | v1 RPC returns 200 not 202 | `respond_v1_async_rpc` | IMP-025 | **DONE** default 202 phase 30 |
| SEC-001 | P0 | Security | Auth secure by default (AC-4) | `config.rs` | IMP-001 | **FIXED** (phase 44 — `auth_enabled: true`; `EDGEQUAKE_DEV_MODE` opt-out) |
| SEC-002 | P0 | Security | Stored API keys broken | `auth_validation.rs` | IMP-002 | **OPT-IN** |
| SEC-003 | P0 | Security | Admin unguarded | `admin.rs` | IMP-003 | **OPT-IN** (E2E baseline) |
| SEC-004 | P0 | Security | Spoofable tenant headers | `middleware.rs` | IMP-004 | **OPT-IN** (E2E strict bind) |
| SEC-005 | P0 | Security | Default JWT secret | `config.rs` | IMP-001 | **OPEN** |
| SEC-006 | P1 | Security | Ollama routes open | `middleware.rs` | IMP-005 | **OPT-IN** |
| SEC-007 | P1 | Security | WebSocket open | `websocket.rs` | IMP-006 | **OPT-IN** (E2E auth-off baseline) |
| SEC-008 | P1 | Security | CORS Any | `server.rs` | IMP-007 | **OPEN** (until env set) |
| SEC-009 | P1 | Security | Rate limit unwired | `routes.rs` | IMP-008 | **OPT-IN** (E2E 429) |
| SEC-011 | P2 | Security | Login lockout not enforced | `login_lockout.rs` | IMP-003 | **FIXED** (phase 32, auth on) |
| SEC-013 | P2 | Security | Dual isolation | `isolation_context.rs` | IMP-023 | **FIXED** (dual mode documented) |
| SEC-014 | P2 | Security | RLS pool session leak | `rls.rs` | IMP-004 | **IMPROVED** (phase 37 SSOT + deprecated legacy) |
| PERF-001 | P0 | O(n) | N+1 entity list degrees | `entity_crud.rs` | IMP-015 | **FIXED** |
| PERF-002 | P0 | O(n) | BFS neighborhood serial | `entity_neighborhood.rs` | IMP-016 | **FIXED** |
| PERF-003 | P0 | O(n) | Full KV keys() bulk ops | `bulk_ops/mod.rs` | IMP-017 | **FIXED** (wsdoc index + SSOT scan) |
| PERF-008 | P2 | O(n) | Graph search neighbor N+1 degrees | `search.rs` | IMP-015 | **FIXED** (phase 16 batch) |
| PERF-008b | P2 | O(n) | Admin user update keys() | `user_management.rs` | IMP-029 | **FIXED** (phase 12 prefix scan) |
| PERF-004 | P1 | O(n) | Document list keys_like | `list.rs` | IMP-019 | **FIXED** |
| PERF-005 | P1 | O(n) | Cost summary scan | `cost_aggregation.rs` | IMP-021 | **FIXED** |
| PERF-006 | P1 | O(n) | Traversal no push-down | `traversal.rs` | IMP-022 | **FIXED** |
| PERF-007 | P2 | O(n) | merge_entities serial graph ops | `entity_merge.rs` | IMP-015 | **FIXED** (phase 18 batch read+write) |
| PERF-CP-001 | P2 | O(n) | Checkpoint cleanup keys_like | `pipeline_checkpoint.rs` | IMP-029 | **FIXED** (phase 17 suffix + batch get) |
| PERF-007b | P2 | O(n) | Workspace delete keys() | `workspace_crud.rs` | IMP-029 | **FIXED** |
| PERF-KV-001 | P1 | O(n) | Global `-metadata` suffix scan | `workspace_document_index.rs` | IMP-029 | **FIXED** (phases 8–10) |
| PERF-KV-002 | P2 | O(n) | Query filter metadata load | `document_filter_resolver.rs` | IMP-029 | **FIXED** (phase 18 scoped SSOT) |
| ARCH-001 | P1 | DRY | Dual isolation SSOT | `isolation_context.rs` | IMP-023 | **FIXED** (dual mode explicit) |
| ARCH-003 | P2 | DRY | Tenant guard boilerplate | `tenant_guard.rs` | IMP-024 | **FIXED** |
| ARCH-004 | P2 | DRY | Cost aggregation dup | `cost_aggregation.rs` | IMP-029 | **FIXED** |
| ARCH-005 | P4 | DRY | Raw `-metadata` format! literals | handlers/services | IMP-029 | **FIXED** (phase 11 key SSOT) |
| ARCH-D-001 | P1 | SOLID | Auth extractors unused | `handlers/auth/extractors.rs` | IMP-003 | **FIXED** phase 30 (login/refresh/logout + get_me) |
| ARCH-I-001 | P2 | SOLID | Full AppState everywhere | `runtime_extractors.rs` | IMP-029 | **FIXED (pattern)** 36 ISP handlers phase 31 |
| ARCH-S-001 | P2 | SOLID | God handlers | `injection/`, `storage_helpers.rs` | IMP-029 | **FIXED** (phase 6) |
| ARCH-S-002 | P2 | SOLID | Metadata write drift | `workspace_document_index.rs` | IMP-029 | **FIXED** (phase 9 write SSOT) |
| ARCH-006 | P2 | DRY | Graph edge DTO mapping | `graph_types.rs` | IMP-029 | **FIXED** (phase 24 SSOT) |

---

## Outlier Files (Post Phase 6)

God files split: `injection/` max 376 LOC; `storage_helpers.rs` facade 131 LOC. **No file >400 LOC** in audited handlers (code-verified phase 6).

---

## Contract Test Coverage (Honest — phase 52)

| Test file | Count | Proves |
|-----------|-------|--------|
| `spec027_api_contract.rs` | 117 pass + 1 ignored | OAuth route lock + auth KV two-callers |
| `spec017_api_contract.rs` | 9 pass | FromRef ISP + AuthState + Compliance/Postgres/AppConfig |
| `spec027_e2e.rs` | 35 pass | +login lockout E2E |
| `spec027_pg_auth_e2e.rs` | 6 pass | PG identity + session + OAuth `/health` |

**Does NOT prove:** load testing, fuzzing, compile-time macro SSOT for `openapi.rs` handler list.

---

## Code Re-assessment (phase 52 — AUTH CLOSED)

IMP-026 auth KV consolidation **DONE**. `auth_kv_store` reachable only from `identity_storage` + `session_storage`. Handlers isolated. Migration **063**. 116+1 contract + 6 pg e2e.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Implementation Phase Map

| Phase | Theme | Outcome |
|-------|-------|---------|
| 0–1 | Production safety flags | OPT-IN mitigations |
| 2 | OpenAPI + DRY/E2E | Route CI, pagination |
| 3–4 | Delete planner + isolation | workspace_crud SSOT, dual mode |
| 5–6 | Entity SSOT + splits | normalize, v2 list, injection module |
| 7 | Entity lookup + v2 cancel | `entity_graph_lookup`, cancel E2E |
| 8 | wsdoc read + migration 047 | prefix scan, bootstrap backfill |
| 9 | wsdoc write SSOT | `upsert_metadata_kv_with_index` (12 paths) |
| 10 | Query filter + stats reads | workspace-scoped wsdoc |
| 11 | Metadata key DRY | `metadata_key_for_document` in all `src/` handlers |
| 12 | Admin user prefix scan | no production `keys()` in handlers |
| 13 | OpenAPI A | bidirectional CI, servers, enrichment |
| 14 | OpenAPI A+ | annotation sync, AsyncAPI sidecar, E2E JSON |
| 15 | OpenAPI A++ | examples, build SSOT, AsyncAPI file, OAS-009 |
| 17 | Reliability + cold paths | merge batch, checkpoint cleanup, reliability contracts |
| 25 | API-SOLID-I-001 FromRef ISP | relationships module |
| 26 | REST-025 + ARCH-D-001 + ISP bulk | opt-in v1 202; admin extractors; lineage |
| 27 | Strict startup bundle + auth complete | create_user ISP |
| 28 | GraphQueryRuntime ISP + OpenAPI 202 | graph handlers + api_keys auth |
| 29 | graph_stream ISP + get_me auth | legacy graph adapters removed |
| 30 | Default REST-025 202 + auth session ISP | list/scan document query ISP |
| 31 | get_document ISP | document query module complete |
| 32 | SEC-011 login lockout | 423 when auth on |
| defer | Secure defaults, ID unification | See 009 |

---

## Code Re-assessment (phase 37)

SEC-014 **IMPROVED**: `acquire_rls_connection` / `release_rls_connection` SSOT; legacy `RlsContext` deprecated. 87+1 contract. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 32)

SEC-011 FIXED: `login_lockout.rs`. 79+1 contract + 33 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 31)

get_document ISP. ARCH-I-001 now 36 handlers. 78+1 contract. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 30)

Default 202. Auth session ISP (login/refresh/logout + ComplianceRuntime). list/scan ISP. 77+1 contract + 32 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 29)

graph_stream ISP. get_me auth. track_status ISP. Legacy graph adapters removed. 75+1 contract + 31 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 28)

GraphQueryRuntime ISP. OpenAPI 202 on all 6 v1 RPC. api_keys auth extractors. 19 ISP handlers. 75+1 contract + 31 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 27)

REST-025 strict bundle. ARCH-D-001 **FIXED**. 15 ISP handlers. 75+1 contract + 31 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 26)

REST-025 **FIXED (opt-in)**. ARCH-D-001 **FIXED (pattern)**. ARCH-I-001 expanded to 7 handlers (lineage). 74+1 contract + 31 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 25)

ARCH-I-001 / API-SOLID-I-001 **FIXED (pattern)** — `runtime_extractors.rs`; relationships module ISP. 73+1 contract + 30 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 24)

ARCH-006 **FIXED** — `GraphEdgeResponse::from_storage_edge`. 72+1 contract + 30 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 23)

All 6 v1 RPC have `run_*` + migration surface. v2 catalog SSOT validation. 71+1 contract + 30 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 22)

REST-024 **FIXED**. v2 catalog scope **FIXED**. `run_*` SOLID partial. 71+29 tests. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 21)

REST-021/022 v1 migration hints **DONE**. 003 v1/v2 split. 68+27 tests. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 20)

REST-001 **DONE (Level 4 v2)**. Flat v2 routes removed. 66+27 tests. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 19)

REST-001 **DONE** on v2 surface (catalog + 5 routes). IMP-025 **DONE**. 65+27 tests. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 18)

PERF-007 **FIXED** (batch read+write). PERF-KV-002 **FIXED** (scoped SSOT). `entity_merge` service. 64+26 tests.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 17)

PERF-007 merge **MITIGATED** (batch writes). PERF-CP-001 **FIXED**. Reliability SSOT contract test added. **005** reliability table. 62 contract tests.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 16)

PERF-008 graph search neighbor degrees **FIXED** — `search.rs` batches via `node_degrees_batch`; contract `spec027_search_nodes_uses_batch_degrees`. O(n) lens **A++** (005). No new migrations.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 15)

OAS-009/010/011 **FIXED**. Build SSOT + 100% DTO examples. OpenAPI **A++** (002).

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

Detail: [010](./010-implementation-plan-phase2.md) | [008](./008-improvement-plan-ascending.md) | Verdict: [009](./009-code-is-law-verdict.md)

---

**Re-assessed:** 2026-06-28 (phase 41)

---

## Code Re-assessment (phase 41)

PG auth E2E proves production auth path. `with_optional_pg_rls` added for handler wiring.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 43)

SEC-014 improved: identity + session PG paths use RLS envelope. Migration 054. SEC-014 ops handlers still OPEN.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 44)

| Finding | Status | Notes |
|---------|--------|-------|
| AC-4 secure defaults | **DONE** | `auth_enabled: true`; `EDGEQUAKE_DEV_MODE` |
| Migration 055 | **DONE** | marker |

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 45)

| Finding | Status | Notes |
|---------|--------|-------|
| IMP-026 KV consolidation | **DONE** | `auth_kv_store.rs` |
| Migration 056 | **DONE** | marker |

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 46)

| Finding | Status | Notes |
|---------|--------|-------|
| KV mirror deprecated | **DONE** | startup warn; migration 057 |
| Health ops SSOT | **DONE** | `health_schema.rs` |
| Dual KV+PG auth | **ELIMINATED** | PG SSOT when pool |

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 47)

| Finding | Status | Notes |
|---------|--------|-------|
| KV mirror ignored with pool | **DONE** | `IdentityPolicy`; migration 058 |
| Dual KV+PG auth | **ELIMINATED** | no read/write when pool |
| Auth scope | **COMPLETE** | 5 PG E2E |

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Historical findings (superseded — do not cite)

| Finding | Status | Notes |
|---------|--------|-------|
| IMP-026 KV auth SSOT | **DONE** | PG-only when pool |
| SEC-014 RLS ops handlers | **OPEN** | health global tables by design |
| AC-4 secure defaults | **DONE** | phase 44 |
| Migration 053 | **DONE** | PG-only auth reads marker |

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 14)

Added OAS-010 (handler annotation drift **FIXED**), ARCH-006, updated test counts to 52+24. OpenAPI lens **A+** per 002. No new migrations.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

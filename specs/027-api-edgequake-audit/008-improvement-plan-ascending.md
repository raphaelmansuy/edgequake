# Improvement Plan — Ascending Compatibility

**Spec:** 027-api-edgequake-audit  
**Last updated:** 2026-06-28 (phase 52 — AUTH scope closed per 009)  
**Authority:** [009-code-is-law-verdict.md](./009-code-is-law-verdict.md) supersedes optimistic pass summaries.

---

## IMP-000: Ascending Compatibility Contract

Shipped changes honor AC-1..AC-7: v1 paths preserved, security **secure-by-default with dev opt-out** (AC-4 phase 44), v2 additive, migrations additive (048–063).

---

## Honest Implementation Status

| Bucket | Count | Meaning |
|--------|-------|---------|
| **DONE** | 25 IMPs | Runtime fix regardless of env |
| **OPT-IN** | 7 IMPs | Code exists; inactive until env set |
| **PARTIAL** | 1 IMP | IMP-026 ID spaces |
| **NOT DONE** | 0 IMPs in engineering scope | Secure defaults = product decision |

Phases 2–16 closed metadata/wsdoc SSOT + **OpenAPI A++** + **O(n) A++** (search batch degrees).

---

## IMP Scorecard (Full)

See [009-code-is-law-verdict.md](./009-code-is-law-verdict.md) for evidence citations.

| ID | Status | Summary |
|----|--------|---------|
| IMP-001 | **DONE** (phase 44) | Secure-by-default `auth_enabled: true`; `EDGEQUAKE_DEV_MODE` opt-out |
| IMP-002 | DONE | KV API key validation when auth on |
| IMP-003 | OPT-IN | Admin guard; bypass when auth off |
| IMP-004 | OPT-IN | JWT/header merge; strict bind opt-in |
| IMP-005 | DONE | Ollama compat gate |
| IMP-006 | OPT-IN | WebSocket auth when auth on |
| IMP-007 | OPT-IN | CORS allowlist |
| IMP-008 | OPT-IN | Rate limit layer |
| IMP-010 | DONE | OpenAPI registry expansion |
| IMP-011 | **DONE** | Triple parity: routes ↔ openapi ↔ utoipa annotations |
| IMP-012–014 | DONE | Models path, SSE, security addon |
| IMP-015–016 | DONE | Batch degrees, neighborhood, **search expand (phase 16)** |
| IMP-017 | DONE | `load_workspace_documents` + wsdoc index |
| IMP-018 | OPT-IN | Bulk delete confirm header |
| IMP-019 | DONE | List scan + pagination |
| IMP-020 | **DONE** | `list_pagination.rs` honors query params |
| IMP-021–022 | DONE | Cost aggregation, traversal push-down |
| IMP-023 | DONE | Dual IsolationMode documented + tested |
| IMP-024 | DONE | Tenant guard DRY |
| IMP-025 | **DONE (L4)** | workspace jobs + submission dispatch + DELETE cancel |
| IMP-026 | PARTIAL | entity_graph_lookup SSOT; ID unification open |
| IMP-027 | DONE | Share URL SSOT |
| IMP-028 | DONE | problem+json Content-Type; hybrid body (AC-2) |
| IMP-029 | **DONE** | God-file splits + wsdoc index read/write SSOT + migration 047 |

---

## Pass History (What Each Pass Actually Did)

| Pass | Delivered | Did NOT deliver |
|------|-----------|-----------------|
| 0–1 | Startup security, auth_validation, admin guard wiring | Secure defaults |
| 2 | OpenAPI expansion, SSE, security addon, route CI | Ollama paths in OpenAPI |
| 3 | Batch graph reads, suffix scans | wsdoc index (later) |
| 4 | isolation_context, tenant_guard, migration 046 | Single isolation semantics |
| 5–6 | Share URL, error fields, v2 list, god-file splits | Full REST-001 |
| 7 | entity_graph_lookup, v2 cancel E2E | IMP-026 breaking unification |
| 8 | wsdoc read index, migration 047 | All write paths synced |
| 9 | `upsert_metadata_kv_with_index` on 12 paths | Secure defaults |
| 10 | query filter + stats wsdoc reads | tenant-only filter global scan (by design) |
| 11 | metadata key DRY in 17 handler files | injection namespace excluded |
| 12 | admin user prefix scan SSOT | no production handler `keys()` |
| 13 | OpenAPI A | bidirectional CI, servers, version, WS extensions |
| 15 | OpenAPI A++ | examples, build SSOT, AsyncAPI file, OAS-009 |
| 16 | O(n) A++ | search.rs neighbor batch degrees |
| defer | IMP-026 breaking, auth-on-by-default | Requires AC-4 break |

---

## Verification Matrix (Honest)

| Check | Result | Caveat |
|-------|--------|--------|
| `spec027_api_contract` | 77 pass + 1 ignored ✅ | default 202 + auth/list/scan ISP |
| `spec027_e2e` | 32 pass ✅ | default 202 + legacy opt-out |
| IMP-028 problem+json | ✅ | Content-Type E2E |
| IMP-029 workspace delete | ✅ | wsdoc + suffix fallback |
| Migration 047 bootstrap | ✅ | idempotent backfill |
| IMP-011 OpenAPI CI | ✅ | routes ⊆ openapi paths |

---

## Definition of Done — Revised

| Criterion | Status |
|-----------|--------|
| Ascending-compat mitigations shipped (AC-1..AC-7) | ✅ |
| Code paths for production hardening exist | ✅ |
| Production secure **by default** | ❌ |
| All P0/P1 findings closed at default config | ❌ |
| Metadata scan SSOT on **all** HTTP read paths | ✅ (indexed + fallback) |
| Metadata write SSOT on production final-metadata paths | ✅ (phase 9) |
| Metadata read SSOT on workspace-scoped consumers | ✅ (phase 10) |
| OpenAPI routes ⊇ Axum router (CI) | ✅ |
| OpenAPI routes ⊆ Axum router (no phantoms) | ✅ (phase 13) |
| OpenAPI handler annotation parity | ✅ (phase 14) |
| OpenAPI E2E JSON endpoint | ✅ (phase 14) |
| RFC 7807 compliant errors (full body) | ❌ (hybrid) |
| Document list pagination | ✅ |
| God-handler splits | ✅ |
| wsdoc workspace index | ✅ |
| v2 REST job catalog + HATEOAS | ✅ (phase 19) |
| No production handler full `keys()` scan | ✅ (phase 12) |

**Verdict:** SPEC-027 ascending-compat engineering shipped (phases 0–30). v2 **Level 4 REST**; v1 **A** (default 202); Rust/DRY/SOLID **A++**.

---

## Code Re-assessment (phase 21)

v1 RPC responses carry `v2_migration`. REST lens v1/v2 split. 68+27 tests.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 20)

Workspace-scoped v2 only. `submission.rs` dispatch. Flat routes removed. 66+27 tests.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 19)

`job_registry.rs` SSOT. `GET /api/v2/jobs/catalog`. JobLinks HATEOAS. IMP-025 **DONE**. 65+27 tests.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 18)

`entity_merge.rs` batch read+write. Query filter scoped SSOT. 64+26 tests. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 17)

merge_entities → `upsert_edges_batch`. Checkpoint cleanup → `keys_with_suffix` + `get_by_ids`. Reliability contract test. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 16)

IMP-015 extended to graph search neighbor expansion — `node_degrees_batch` in `search.rs`. Contract test added. **005 → A++**. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 15)

`openapi_examples.rs` (100% DTO examples), `build.rs` path SSOT, standalone `/api-docs/asyncapi.json`, OAS-009 codegen script + snapshot, utoipa `=5.4.0` pin. **002 → A++**.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 14)

`openapi_annotation_sync.rs` enforces every OpenAPI path has a handler `#[utoipa::path]` annotation. AsyncAPI sidecar at `x-edgequake-asyncapi`. Swagger UI `persist_authorization(true)`. E2E validates live `/api-docs/openapi.json`. **002 → A+**.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 13)

IMP-011 extended to **bidirectional** parity: `openapi_phantom_paths` detects spec entries with no Axum route. `openapi_enrichment.rs` adds `servers`, syncs `info.version` to `CARGO_PKG_VERSION`, and tags WebSocket ops with `x-edgequake-transport`. **002-openapi-swagger-lens.md** upgraded to **A**.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 12)

`user_management.rs` last-admin demotion guard now uses `count_other_admin_users` → `list_user_record_keys` → `keys_with_prefix(USER_KEY_PREFIX)`. Eliminates the only production `kv_storage.keys().await` in handlers. Contract test `spec027_user_management_no_full_kv_keys_scan` enforces.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 11)

All production handlers in `edgequake-api/src/` now delegate document metadata key construction to `metadata_key_for_document` (delegates to `kv_keys::doc_metadata`). Contract test `spec027_document_metadata_key_uses_dry_helper` walks `src/` and forbids raw `format!("{}-metadata")`. Injection keys (`injection::…-metadata`) intentionally excluded.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 10)

IMP-029 consumer coverage extended to `document_filter_resolver` (workspace-scoped) and `workspaces/stats.rs`. Tenant-only query filter intentionally retains global suffix scan for non-UUID tenant string ids.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 32)

SEC-011 login lockout. 79+1 contract + 33 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 31)

get_document ISP. Document query module complete (4/4). 78+1 contract + 32 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 30)

IMP-025 default 202 **DONE**. Auth session ISP. list/scan ISP. 77+1 contract + 32 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 29)

IMP-003 extended to get_me. graph_stream ISP. Legacy adapters removed. 75+1 contract + 31 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 28)

IMP-025 OpenAPI 202 on 6 RPC **DONE**. IMP-003 **DONE** (api_keys). GraphQueryRuntime ISP. 75+1 contract + 31 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 27)

REST-025 strict-startup bundle **DONE**. ARCH-D-001 **DONE**. IMP-003 **DONE** (create_user). 75+1 contract + 31 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 26)

REST-025 **DONE (opt-in)**. ARCH-D-001 **FIXED (pattern)**. IMP-003 **PARTIAL**. 74+1 contract + 31 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 25)

API-SOLID-I-001 **FIXED**. 73+1 contract + 30 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 24)

ARCH-006 graph edge DTO SSOT. 72+1 contract + 30 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 23)

`run_reanalyze_multimodal` completes 6/6 RPC SOLID extract. `is_creatable_v2_job_type` SSOT. 71+1 contract + 30 e2e. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 22)

REST-024 Sunset/Link on v1 RPC. v2 catalog scope. `run_*` inner functions for 5 RPC types. 71+29 tests. No migration.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Environment Reference

`.env.example` SPEC-027 section — minimum for internet-facing:

```bash
EDGEQUAKE_AUTH_ENABLED=true
EDGEQUAKE_STRICT_STARTUP=1
EDGEQUAKE_STRICT_TENANT_BIND=1
EDGEQUAKE_CORS_ORIGINS=https://your-ui.example
EDGEQUAKE_RATE_LIMIT_ENABLED=true
EDGEQUAKE_REQUIRE_DELETE_ALL_CONFIRM=true
```

---

## Tests

```bash
cargo test -p edgequake-api --test spec027_api_contract --test spec027_e2e
cargo test -p edgequake-api --test migration_bootstrap_proof  # needs DATABASE_URL
```

**Current:** 94 contract + 1 ignored + 33 e2e + 2 pg auth e2e (phase 41).

---

## Code Re-assessment (phase 41)

IMP-026 narrowed further — PG auth E2E when DATABASE_URL set. `with_optional_pg_rls` for SEC-014 handler wiring.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 43)

IMP-026: KV auth helpers remain for in-memory tests only. Production PG envelope complete (migration 054). AC-4 still OPEN.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 40)

IMP-026 narrowed: production auth is PG-only reads; KV auth path is test harness only. Migration 053 documents authority. IMP-004 unchanged.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 37)

IMP-004 (tenant bind) unchanged. RLS storage SSOT landed in `rls.rs` — no new IMP; SEC-014 improved via engineering not env flags. Dual KV+PG **keep** (bootstrap 048–050 does not justify PG-only cutover).

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 44)

AC-4 **DONE**: auth on by default; `EDGEQUAKE_DEV_MODE` ascending-compat for local dev.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 45)

IMP-026 **DONE**: `auth_kv_store.rs` consolidates KV auth. Migration 056.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 46)

KV mirror **deprecated** (057). `health_schema.rs`. Auth/identity IMPs **complete**.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 47)

KV mirror **ignored** with pool (058). All auth IMPs **closed**.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

# SPEC-027 — EdgeQuake API Multi-Lens Audit

**Spec:** `027-api-edgequake-audit`  
**Date:** 2026-06-28 (post phase 54 — OIDC opt-in)  
**Method:** Code is law — [009-code-is-law-verdict.md](./009-code-is-law-verdict.md) is authoritative.  
**Scope:** `edgequake-api` HTTP surface + auth/storage coupling  

---

## Executive Verdict (Brutal, Post Phase 54)

EdgeQuake ships ~100+ live HTTP routes. **Auth/identity engineering closed** at phase 52. **Builtin OIDC opt-in** at phase 54.

| Claim | Reality (code-verified) |
|-------|-------------------------|
| "SPEC-027 auth complete" | **True** — PG SSOT; handlers + service layer locked |
| "OAuth2/OIDC in EdgeQuake" | **Opt-in** — env-gated PKCE routes; default **false** |
| "Handlers use auth_kv_store" | **False** — only `identity_storage` + `session_storage` |
| "KV auth SSOT in production" | **False** — never read/written with pool |
| Contract honesty | **A** — 117 contract + 2 ignored + 35 e2e + **7** oidc e2e + **6** pg |

**Bottom line:** PostgreSQL is identity/RBAC/session SSOT. KV `auth:*` is never consulted for identity when pool exists (OIDC pending state is ephemeral KV only).

---

## Implementation Phases (Code-Verified)

| Phase | Theme | Outcome |
|-------|-------|---------|
| 0–1 | Production safety flags | OPT-IN mitigations |
| 2–4 | OpenAPI + isolation + bulk SSOT | Route CI, pagination, dual isolation |
| 5–6 | Entity SSOT + god-file splits | normalize, v2 list, injection module |
| 7 | Entity lookup + v2 cancel | `entity_graph_lookup`, cancel E2E |
| 8 | wsdoc read index + migration 047 | O(workspace) prefix scan + bootstrap backfill |
| 9 | wsdoc write SSOT | `upsert_metadata_kv_with_index` on 12 paths |
| 10 | Remaining read consumers | query filter + workspace stats wsdoc |
| 11 | Metadata key DRY | `metadata_key_for_document` everywhere in `src/` |
| 12 | Admin user prefix scan | no production `keys()` |
| 13 | OpenAPI A | bidirectional CI, servers, enrichment |
| 14 | OpenAPI A+ | annotation sync, AsyncAPI sidecar, E2E JSON |
| 15 | OpenAPI A++ | examples, build SSOT, standalone AsyncAPI, OAS-009 |
| 16 | O(n) A++ | graph search neighbor `node_degrees_batch` |
| 17 | Reliability + cold paths | checkpoint suffix, merge batch writes |
| 18 | P2 cold paths A++ | merge batch reads, query filter SSOT, entity_merge service |
| 19 | v2 REST A++ (flat routes) | superseded by phase 20 |
| 20 | v2 Level 4 REST | workspace-scoped jobs, DELETE cancel |
| 21 | v1 migration hints + doc split | `v2_migration` field, OAS extensions, 003 rewrite |
| 22 | REST-024 + v2 scope + SOLID | Sunset/Link headers, catalog scope, `run_*` extract |
| 23 | run_reanalyze + catalog SSOT | 6/6 `run_*`, job type validation, v2 Link header |
| 24 | ARCH-006 graph edge DTO SSOT | `GraphEdgeResponse::from_storage_edge` |
| 25 | API-SOLID-I-001 FromRef ISP | `runtime_extractors.rs`; relationships module |
| 26 | REST-025 + ARCH-D-001 + ISP bulk | opt-in v1 202; admin extractors; lineage ISP |
| 27 | Strict startup bundle + auth complete | create_user ISP; 15 storage handlers |
| 28 | GraphQueryRuntime ISP + OpenAPI 202 | graph handlers + api_keys auth |
| 29 | graph_stream ISP + get_me auth | track_status ISP |
| 30 | Default REST-025 202 + auth session ISP | list/scan document query ISP |
| 31 | get_document ISP | document query module complete |
| 32 | SEC-011 login lockout | 423 ACCOUNT_LOCKED when auth on |
| 33 | Identity storage SSOT | KV→PG sync + migration 048 + SEC-010 |
| 34 | Memberships wired | PG sync + JWT claims + migration 049 |
| 35 | Tenant isolation SSOT | 3 layers + pool-safe RLS + migration 050 |
| 36 | Conversation RLS acquire | `conversation.rs` acquired-connection RLS; SEC-014 improved |
| 37 | RLS SSOT DRY | `acquire_rls_connection`; legacy pool API deprecated |
| 38 | PG identity SSOT primary | `IdentityPolicy` + RLS default on + migration 051 |
| 39 | PG session artifacts SSOT | `session_storage` refresh + api_keys + migration 052 |
| 40 | PG-only auth reads | `kv_auth_reads_enabled`; no KV fallback when pool |
| 41 | PG auth E2E + RLS helper | `spec027_pg_auth_e2e` + `with_optional_pg_rls` |
| 42 | Handler RLS (pdf_documents) | `pdf_lineage` + logout PG E2E |
| 43 | Identity/session PG envelope | migration 054 |
| 44 | AC-4 secure by default | `EDGEQUAKE_DEV_MODE`; migration 055 |
| 45 | IMP-026 KV consolidation | `auth_kv_store.rs`; migration 056 |
| 46 | KV mirror deprecated | startup warn; `health_schema`; migration 057 |
| 47 | KV mirror **ignored** with PG pool | `IdentityPolicy`; migration 058 |
| 48 | Explicit PG vs KV branches | DRY `pg_primary` / else; migration 059 |
| 49 | OAuth2/OIDC honesty + KV quarantine | `/health` flags; migration 060 |
| 50 | Handler isolation (`user_management`) | migration 061 |
| 51 | Full `handlers/auth` isolation | migration 062 |
| 52 | Service-layer SSOT lock — **AUTH CLOSED** | migration 063 |
| 53 | OAuth route lock (historical) | superseded by 54 |
| 54 | Builtin OIDC opt-in (PKCE) | migration 064 |
| 55 | **KV auth eliminated** — `AuthMemoryStore` | migration 065 |

---

## Document Map

| Doc | Purpose | Re-assessed |
|-----|---------|-------------|
| **[009-code-is-law-verdict.md](./009-code-is-law-verdict.md)** | **Authority** | ✅ phase 55 |
| [010-implementation-plan-phase2.md](./010-implementation-plan-phase2.md) | Phase 2–55 execution | ✅ phase 55 |
| **[004-security-oauth-lens.md](./004-security-oauth-lens.md)** | **Security + OAuth + isolation** | ✅ phase 55 |
| [003-rest-design-lens.md](./003-rest-design-lens.md) | REST v1/v2 lens | ✅ phase 52 (auth N/A) |
| [005-complexity-system-lens.md](./005-complexity-system-lens.md) | O(n) + reliability | ✅ phase 52 (auth N/A) |
| [002-openapi-swagger-lens.md](./002-openapi-swagger-lens.md) | OpenAPI lens | ✅ phase 52 |
| [006-rust-architecture-lens.md](./006-rust-architecture-lens.md) | DRY/SOLID lens | ✅ phase 52 |
| [008-improvement-plan-ascending.md](./008-improvement-plan-ascending.md) | IMP tracking | ✅ phase 52 |
| [007-cross-reference-matrix.md](./007-cross-reference-matrix.md) | Finding matrix | ✅ phase 52 |
| [001-first-principles-methodology.md](./001-first-principles-methodology.md) | Methodology | ✅ phase 52 |

---

## Verification Commands

```bash
cargo test -p edgequake-api --features postgres \
  --test spec027_api_contract --test spec027_e2e --test spec027_oidc_e2e --test spec027_pg_auth_e2e
cargo clippy -p edgequake-api --features postgres -- -D warnings
```

**Current:** 117 contract + 2 ignored + 35 e2e + **7** oidc e2e + 6 pg ✅ (phase 54 — OIDC opt-in COMPLETE)

---

## Task Log Reference

`/logs/2026-06-28-*-beastmode-chatmode-log.md`

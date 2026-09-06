# 03 — Code As-Is (SPEC-146)

Ground truth for gaps. Paths relative to repo root unless noted.

## Identity & RBAC

| Concern | Path | Behavior today |
|---------|------|----------------|
| Roles | `edgequake/crates/edgequake-auth/src/types.rs` | `Role::{Admin, User, Readonly}` |
| Permissions | `edgequake/crates/edgequake-auth/src/rbac.rs` | Full `Permission` enum; **zero** `require_permission` call sites in `edgequake-api` |
| JWT | `edgequake-auth/src/jwt.rs` | Claims: `sub`, `role`, optional `tenant_id`/`workspace_id` |
| Middleware | `edgequake-api/src/middleware.rs` | `protected_api_auth`, `TenantContext` from headers |
| Validation | `edgequake-api/src/services/auth_validation.rs` | Master key → JWT → stored `eq_` keys |
| Identity SSOT | `edgequake-api/src/services/identity_storage.rs` | PG users; login stamps **default** tenant/workspace UUIDs |
| Membership | `edgequake-core/.../membership.rs` | `owner/admin/member/readonly`; UNIQUE `(user_id, tenant_id, workspace_id)` |
| Strict bind | `EDGEQUAKE_STRICT_TENANT_BIND` | Default **false** |
| RLS | migrations `009`, `096` | Tenant/workspace GUCs; fail-closed FORCE RLS |

```ascii
  Request
    │
    ├─ Authorization: Bearer | X-API-Key
    ├─ X-Tenant-ID / X-Workspace-ID
    ▼
  validate_presented_token ──► RequestAuthContext { user_id, role }
    │
    ▼
  TenantContext (headers + overwrite user_id)
    │
    ▼
  Handlers ──► workspace filters only
               (no document ACL)
```

## Tables (identity)

| Table | Notes |
|-------|-------|
| `users` | `role` CHECK admin/user/readonly |
| `tenants` / `workspaces` | Hierarchy |
| `memberships` | Workspace roles |
| `api_keys` | Scopes → Admin if contains `"admin"`, else User |
| `refresh_tokens` | Session |
| `audit_logs` | Exists (migration 012); break-glass must use loudly |

**Missing:** `document_acl`, `attribute_definitions`, `principal_attributes`, `workspace_roles`, `policies`, `policy_versions`.

## Documents & upload

| Concern          | Path                                                                                                                                             |
| ------------------| --------------------------------------------------------------------------------------------------------------------------------------------------|
| Schema           | `edgequake/migrations/001_init_database.sql` — `documents` + `chunks`                                                                            |
| Status evolution | `017`, `032`, `141` — lifecycle incl. `deleting` / `delete_failed`                                                                               |
| Admit INSERT     | `edgequake-api/src/services/ingest_admission.rs`                                                                                                 |
| Metadata nesting | `handlers/documents/upload/document_admission.rs` — client JSON → `custom_metadata`                                                              |
| Text upload      | `handlers/documents/upload/file_upload.rs`                                                                                                       |
| PDF upload       | `handlers/pdf_upload/upload.rs`                                                                                                                  |
| List             | `handlers/documents/query/list.rs` — **KV** `document_metadata_scan` + staging merge; date/pattern/status; **`status_counts` global** (SPEC-084) |
| Detail/download  | `handlers/documents/query/detail.rs`, `download.rs`                                                                                              |
| WebUI dropzone   | `edgequake_webui/src/components/documents/document-dropzone.tsx`                                                                                 |
| PDF FormData     | `edgequake_webui/src/lib/upload/pdf-upload-form-data.ts`                                                                                         |
| Status chips     | `status-badge.tsx` / `enhanced-status-badge.tsx`                                                                                                 |
| User mgmt UI     | `settings/user-management-card.tsx` — roles `admin/developer/viewer`                                                                             |

```ascii
  POST /documents | /upload | /pdf
         │
         ▼
  document_admission
         │  metadata = { title, track_id, status, ... }
         │  custom_metadata = client bag (tags optional, unused by UI)
         ▼
  documents row (pending) + KV staging
         │
         ▼
  Worker pipeline ──► chunks / embeddings / AGE
         │
         ▼
  status completed|indexed|failed|...
         │
         ✗  no classification / share_mode / ACL columns
```

## Query & retrieval

| Mode | Path |
|------|------|
| Dispatch | `edgequake-query/.../query_entry/query_pipeline.rs` |
| Naive/Local/Global/Hybrid/Mix | `engine_impl/modes/*.rs` |
| Scope filter helper | `make_scope_metadata_filter` in `modes/mod.rs` |
| Document filter resolve | `edgequake-api/.../document_filter_resolver.rs` |
| Post-filter | `context_filter.rs` — `filter_context_by_document_ids` |
| Citations | `source_reference_builder`, `citation_verify.rs` |
| Context format | `context_format.rs` — `doc="Title"` |

```ascii
  POST /query|/query/stream
         │
         ▼
  resolve_document_filter ──► Option<ids>  (empty = ALL workspace)
         │
         ▼
  pipeline_retrieve (mode)
         │
         ├─ Typed ANN: workspace_id ONLY  (F-146-04)
         ├─ Graph expand: workspace only  (F-146-06)
         ├─ postprocess allow-list optional
         └─ caches: no principal           (F-146-10)
```

## Vector / embeddings

| Table | `document_id`? |
|-------|----------------|
| `chunks` | Yes (FK) |
| `chunk_embeddings` (108) | **No** — `(model_id, chunk_id, workspace_id, embedding)` |
| `entity_embeddings` / `relationship_embeddings` / `report_embeddings` (130) | **No** |
| Legacy `eq_*_vectors` | Yes (denorm + JSONB) |

**Default backend:** `EDGEQUAKE_VECTOR_BACKEND` unset → `TypedEmbeddings` ([`vector_backend.rs`](../../edgequake/crates/edgequake-storage/src/vector_backend.rs)). Typed path is production authority.

Typed read: `try_typed_chunk_query(pool, index, emb, top_k, workspace_key)` — **no document filter arg**.  
Fleet search: `WHERE fe.workspace_id = $3` only.

## Graph

| Concern | Path |
|---------|------|
| Expand | `edgequake-query/src/graph_expand.rs` |
| BFS hops | `graph_hops.rs` |
| PPR | `graph_ppr.rs` |
| KG→chunk | `kg_chunk_pick.rs` |
| Community | `community_global.rs` |
| HTTP graph | `edgequake-api/src/handlers/graph/**` |
| Popular | `handlers/graph/graph_query/popular.rs` |
| Provenance | AGE/relational `source_ids` / `source_chunk_ids` |

## Caches

| Cache | Key shape (today) | Gap |
|-------|-------------------|-----|
| Keywords | `{mode}:keywords:{hash}-cache` | No principal/workspace/policy |
| Answer | `{mode}:query:{hash}-cache` | Prompt hash only |
| Context | `ws:{workspace}:ctx:{hex}` includes `allowed_document_ids` when `Some` | Default **`None`** → shared across users |
| MCP ret | `ret_*` | Unbound to principal |
| `llm_cache` table | `cache_key`, `namespace` | No user/doc columns |

## MCP

| Concern | Path |
|---------|------|
| Auth | `edgequake-api/src/mcp/auth/` |
| Tools | `mcp/gateway/tools.rs` — search / fetch / retrieve |
| Workspace policy | `workspace_policy.rs` — weak claim check |
| Dispatch | `mcp/gateway/dispatch.rs` |

## Parse

| Endpoint | Gap |
|----------|-----|
| `POST /api/v1/parse` | Under auth when enabled; `_context` unused for document ACL |
| `GET /api/v1/parse/jobs/{id}` | **IDOR** — not public, but any authenticated principal can fetch any job UUID (F-146-20) |

## Audit (existing crate)

| Concern | Path |
|---------|------|
| Crate | `edgequake/crates/edgequake-audit/` |
| Types | `AuditEventType::{Authorization, DocumentQuery, …}` |
| Reuse | Break-glass + authz denials — **extend**, do not fork |

## Master / API key identity

| Principal | Today |
|-----------|-------|
| Master / env key | `user_id = "master-api-key"` (string, **not** UUID) |
| Stored `eq_` key | Owner `user_id` UUID; scopes → Admin vs User only |
| Worker | No distinct principal kind on task claim |

→ LAW-146-19 tagged `PrincipalId`.

## OpenAPI surfaces (no members)

Documents, query, graph, users, tenants, workspaces, api-keys, auth — **no** `/api/v1/.../members`.

## WebUI role drift

```ascii
  UserManagementCard.ROLES = admin | developer | viewer
  API users.role           = admin | user     | readonly
  memberships.role         = owner | admin | member | readonly
```

## Summary ASCII — isolation today

```ascii
  Tenant ──► Workspace ──► Document ──► Chunk ──► Embedding
                 │              │
                 │              └── ✗ no ACL / classification
                 │
                 ├── YES: RLS on SQL tables (not KV list)
                 └── YES: per-ws vector tables

  List path:  KV metadata scan (RLS does not apply)   ← F-146-33
  Query path: workspace wall only; typed ANN no doc ids
  Graph path: workspace wall only
  Cache path: keyword/answer unbound; context None = share
  MCP path:   token + weak workspace claim
  Parse jobs: authenticated IDOR
```

## Cross-refs

- Findings → [01-finding-register.md](01-finding-register.md)  
- Target → [04-target-architecture.md](04-target-architecture.md)  
- Data model → [05-data-model.md](05-data-model.md)  
- Honest → [13-honest-assessment.md](13-honest-assessment.md)  

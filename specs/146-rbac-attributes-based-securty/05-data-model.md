# 05 — Data Model (SPEC-146)

Migrations start after current highest (`149`…): propose **`150_spec146_*`** series. Exact numbers assigned at implement time via `make`/checksum scripts.

## Resource hierarchy

```ascii
  Tenant
    └── Workspace
          ├── Documents (security columns + ACL)
          │     └── Chunks ──► chunk_embeddings (JOIN document_id)
          ├── Entity hubs (unlabeled for secrets)
          │     └── Occurrences / edges (source_ids / provenance)
          ├── Memberships + principal_attributes
          ├── workspace_roles + bindings
          ├── attribute_definitions
          ├── policies / policy_versions
          └── workspace_authz_state (policy_generation)
```

## Documents — new columns

```sql
-- Conceptual DDL (normative intent; polish in migration)
ALTER TABLE documents
  ADD COLUMN classification   TEXT NOT NULL DEFAULT 'internal',
  ADD COLUMN share_mode       TEXT NOT NULL DEFAULT 'workspace',
  ADD COLUMN export_control   BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN pii              BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN project_id       TEXT,
  ADD COLUMN owner_org        TEXT,
  ADD COLUMN owner_principal_kind TEXT NOT NULL DEFAULT 'user',
              -- user | api_key | master | worker  (LAW-146-19)
  ADD COLUMN owner_principal_id   TEXT,  -- Uuid string OR sentinel (master/worker)
                                         -- NOT a bare UUID FK to users
  ADD COLUMN retention_until  TIMESTAMPTZ,
  ADD COLUMN policy_etag      TEXT,  -- content hash of document security labels
                                    -- (NOT workspace policy_generation; LAW-146-22)
  ADD COLUMN security_status  TEXT NOT NULL DEFAULT 'ok',
  ADD COLUMN security_attrs   JSONB NOT NULL DEFAULT '{}';

-- CHECKs
-- share_mode IN ('workspace','acl','classified','owner_only')
-- security_status IN ('ok','quarantined')
-- classification IN catalog values OR free-text constrained by attribute_definitions
```

**Backfill (LAW-146-14):**

```sql
UPDATE documents SET share_mode = 'workspace', security_status = 'ok'
WHERE share_mode IS NULL OR TRUE; -- applied once on add
```

Indexes:

```ascii
  idx_documents_ws_share     (workspace_id, share_mode, security_status)
  idx_documents_ws_class     (workspace_id, classification)
  idx_documents_owner        (owner_principal_kind, owner_principal_id)
  GIN security_attrs         (optional; prefer typed columns for hot filters)
```

## PrincipalId (LAW-146-19)

```ascii
  PrincipalId =
      User(Uuid)      -- users.user_id
    | ApiKey(Uuid)    -- api_keys.key_id (not the hash)
    | Master          -- env/master key; string sentinel "master"
    | Worker          -- ingestion task claim

  Storage columns: (principal_kind TEXT, principal_id TEXT)
  ✗ Do NOT use UUID-only FK to users for ACL rows
```

## document_acl

```sql
CREATE TABLE document_acl (
  document_id     UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
  principal_kind  TEXT NOT NULL,  -- user|api_key|master|worker|group
  principal_id    TEXT NOT NULL,  -- uuid text or sentinel
  permission      TEXT NOT NULL,  -- document:read|write|delete|set_labels
  granted_by_kind TEXT,
  granted_by_id   TEXT,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (document_id, principal_kind, principal_id, permission)
);
CREATE INDEX idx_document_acl_principal
  ON document_acl (principal_kind, principal_id);
```

## Attribute catalog & principal attrs

```sql
CREATE TABLE attribute_definitions (
  attr_id       UUID PRIMARY KEY,
  workspace_id  UUID NOT NULL REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
  scope         TEXT NOT NULL, -- subject|resource|environment
  name          TEXT NOT NULL, -- e.g. clearance, department
  value_type    TEXT NOT NULL, -- string|enum|string_set|bool|int
  enum_values   JSONB,
  required_for_share_modes TEXT[] DEFAULT '{}',
  UNIQUE (workspace_id, scope, name)
);

CREATE TABLE principal_attributes (
  workspace_id    UUID NOT NULL,
  principal_kind  TEXT NOT NULL,
  principal_id    TEXT NOT NULL,
  name            TEXT NOT NULL,
  value           JSONB NOT NULL,
  source          TEXT NOT NULL DEFAULT 'manual', -- manual|oidc|sync
  updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (workspace_id, principal_kind, principal_id, name)
);
```

## Roles (workspace-scoped custom + built-ins)

```sql
CREATE TABLE workspace_roles (
  role_id       UUID PRIMARY KEY,
  workspace_id  UUID NOT NULL REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
  name          TEXT NOT NULL,  -- viewer|editor|admin|ingestion_service|...
  is_builtin    BOOLEAN NOT NULL DEFAULT FALSE,
  permissions   TEXT[] NOT NULL, -- Permission::as_str values
  UNIQUE (workspace_id, name)
);

CREATE TABLE workspace_role_bindings (
  workspace_id    UUID NOT NULL,
  principal_kind  TEXT NOT NULL,
  principal_id    TEXT NOT NULL,
  role_id         UUID NOT NULL REFERENCES workspace_roles(role_id) ON DELETE CASCADE,
  PRIMARY KEY (workspace_id, principal_kind, principal_id, role_id)
);
```

Built-ins seeded per workspace (see [12-role-attribute-catalog.md](12-role-attribute-catalog.md)). Membership `owner/admin/member/readonly` continues to exist; bindings map to capability sets. Document access is **not** stored only as a fourth global `users.role`.

## Policy store (PAP)

Cedar text is used **only** for `share_mode=classified` evaluation and PAP validation (LAW-146-18).

```sql
CREATE TABLE policies (
  policy_id     UUID PRIMARY KEY,
  workspace_id  UUID NOT NULL REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
  name          TEXT NOT NULL,
  active_version BIGINT NOT NULL DEFAULT 0,
  UNIQUE (workspace_id, name)
);

CREATE TABLE policy_versions (
  policy_id     UUID NOT NULL REFERENCES policies(policy_id) ON DELETE CASCADE,
  version       BIGINT NOT NULL,
  cedar_text    TEXT NOT NULL,
  cedar_hash    CHAR(64) NOT NULL, -- sha256 hex
  schema_hash   CHAR(64) NOT NULL,
  created_by    UUID,
  created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (policy_id, version)
);

-- LAW-146-22: single monotonic generation for cache / AuthzContext / audit
CREATE TABLE workspace_authz_state (
  workspace_id      UUID PRIMARY KEY REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
  policy_generation BIGINT NOT NULL DEFAULT 1,
  updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**`policy_generation` vs `policy_etag` (LAW-146-22):**

```ascii
  workspace_authz_state.policy_generation
    │  BIGINT, monotonic per workspace
    │  bump on: policy publish, ACL change, principal attr change,
    │           document security columns / share_mode / quarantine
    │           that can alter allow-sets
    └─► AuthzContext.policy_generation (stamped at request start)
        cache keys, MCP ret_*, audit deny rows

  documents.policy_etag
    │  content hash of that document's security labels
    └─► optimistic concurrency / “labels changed?” — NOT the generation
```

`AuthzContext` carries **`policy_generation`** (not an ambiguous “max of policy_versions.version”).

Bump helper (conceptual):

```sql
UPDATE workspace_authz_state
SET policy_generation = policy_generation + 1, updated_at = NOW()
WHERE workspace_id = $ws;
-- INSERT … ON CONFLICT if row missing (generation starts at 1)
```

## Allow-set size cap (LAW-146-21)

```ascii
  |allow| <= ALLOW_SET_ARRAY_THRESHOLD (default propose 2048; measure)
      → pass uuid[] to ANY($allow) / GUC
  |allow| > threshold
      → stage into TEMP TABLE / UNNEST JOIN / bitmap strategy
      → do NOT push multi-10k arrays blindly into ANY()

  ANN when ABAC on:
      overfetch_k = min(top_k * OVERFETCH_FACTOR, OVERFETCH_CAP)
      OR SET LOCAL hnsw.iterative_scan = 'relaxed_order'
         (+ documented hnsw.max_scan_tuples, default 20000)
      then filter / retain until top_k
  Recall under sparse allow-set = UNCONFIRMED until M2/M5 measure
```

## Chunks / embeddings

**Preferred (LAW-146-9 + LAW-146-21):** JOIN — no mandatory denorm on day 1.

```sql
-- Typed chunk ANN target shape (illustrative; over-fetch then filter in engine)
-- Prefer iterative_scan when PG/pgvector supports it for this fleet.
SELECT ce.chunk_id, ce.embedding <=> $q AS dist
FROM chunk_embeddings ce
JOIN chunks c ON c.id = ce.chunk_id
WHERE ce.model_id = $m
  AND ce.workspace_id = $ws
  AND c.document_id = ANY($allow_set::uuid[])  -- or JOIN temp_allow when large
ORDER BY dist
LIMIT $overfetch_k;  -- then retain top_k after any post-filter
```

Optional denorm `chunk_embeddings.document_id` if JOIN p95 regresses (measure in M2; do not assume).

Fleet entity/rel search must constrain via `entities.source_ids` / relationship provenance ∩ allow-set (or skip entity arm when allow-set empty).

## Break-glass sessions (LAW-146-25)

```sql
CREATE TABLE break_glass_sessions (
  session_id     UUID PRIMARY KEY,
  workspace_id   UUID NOT NULL REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
  principal_kind TEXT NOT NULL,
  principal_id   TEXT NOT NULL,
  reason         TEXT NOT NULL,
  expires_at     TIMESTAMPTZ NOT NULL,  -- default now() + 15 minutes
  scope_doc_ids  UUID[],                -- NULL = all non-quarantined in ws (still TTL-bound)
  created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  revoked_at     TIMESTAMPTZ
);
CREATE INDEX idx_break_glass_active
  ON break_glass_sessions (workspace_id, expires_at)
  WHERE revoked_at IS NULL;
```

Allow-set for break-glass = (scope_doc_ids ∩ non-quarantined) **or** (all non-quarantined in workspace) **and** `now() < expires_at` and `revoked_at IS NULL`. Audit row mirrors TTL + scope.

## AGE / graph

```ascii
  Entity hub node
    - id, name, description (query-time assembled)
    - NO classification label for secrets

  Occurrence / Edge
    - source_ids / source_chunk_ids / workspace_id
    - authz: keep iff source_ids ∩ allow-set ≠ ∅
```

Do **not** materialize per-principal graphs in v1.

## RLS backstop

Workspace RLS stays. Add document-level defense:

```ascii
  Option A (v1 preferred): security-barrier view
    documents_authorized AS
      SELECT d.* FROM documents d
      WHERE d.workspace_id = current_workspace_id()
        AND d.id = ANY(current_setting('app.allow_document_ids')::uuid[])
        -- empty array => no rows (fail-closed when ABAC on)

  Option B: policy using session GUC allow-list set by PEP at TX start
```

App role never BYPASSRLS. Break-glass uses audited session with explicit GUC + audit row + TTL (LAW-146-25).

**Worker DB role (G6 ops):** ingestion worker role has no broad SELECT on `documents`/`chunks` beyond staging/claim tables it owns; RLS + allow-set GUC still apply if worker ever reads content tables. PEP denial of `query.execute` for `PrincipalId::Worker` is necessary but not sufficient against raw SQL.

## Quarantine

```ascii
  security_status = quarantined
    │
    ├─ excluded from AllowSet builder
    ├─ excluded from list for non-editors (editors see with chip)
    ├─ track_id phase: labeling_failed
    └─ retry labels endpoint → set attrs → security_status=ok
```

## Cedar schema sketch

```cedar
entity User {
  clearance: String,
  department: String,
  groups: Set<String>,
  workspace_roles: Set<String>,
};

entity Document {
  classification: String,
  share_mode: String,
  project_id: String,
  export_control: Bool,
  pii: Bool,
  owner_org: String,
  owner: User,
};

entity Workspace;

action document_read appliesTo {
  principal: User,
  resource: Document,
};

// Example forbid (deny-overrides)
forbid (principal, action == Action::"document_read", resource)
when { resource.share_mode == "classified"
       && !(principal.clearance == "secret"
            || principal.clearance == "top_secret") };
```

Full schema + templates live in `edgequake-authz` at implement time; validated on policy publish.

## Inheritance of labels

| Child | Rule |
|-------|------|
| Chunks | Inherit document_id → document security |
| MM assets / figures | Inherit parent document labels (EC-146-19) |
| PDF pages | Same document_id |
| Reprocess | Must not drop or widen labels without `document:set_labels` |
| KV `{uuid}-metadata` | **Dual-write** classification/share_mode/security_status at admit (F-146-36); list PEP reads KV |

## RLS vs list PEP (LAW-146-17)

```ascii
  SQL detail/download/ANN ──► RLS + allow-set GUC = backstop
  KV list/search/autocomplete ──► application PEP only
       RLS does NOT see KV rows
```

## Cross-refs

- Architecture → [04-target-architecture.md](04-target-architecture.md)  
- Catalog → [12-role-attribute-catalog.md](12-role-attribute-catalog.md)  
- DB lens → [05-lenses/003-database.md](05-lenses/003-database.md)  
- Edge cases → [09-edge-cases.md](09-edge-cases.md)  
- Honest → [13-honest-assessment.md](13-honest-assessment.md)  
- Roadblocks → [14-roadblocks.md](14-roadblocks.md)  
